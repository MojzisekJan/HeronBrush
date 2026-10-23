//! Writes a Flax Engine import script beside the meshes.
//!
//! Flax reads glTF through its content pipeline, so the Flax target keeps the `.glb`
//! meshes the level build already makes and generates one C# editor script with the
//! placement baked in, the way the Unity and Unreal targets do. Dropping the exported
//! folder into the project imports the meshes; running the script then spawns a
//! `StaticModel` per solid at its place in the open scene.
//!
//! Baking the placements into the script rather than writing a `.scene` is deliberate.
//! A Flax scene is JSON, but it is the engine's own serialized form -- every actor keyed
//! by a `Guid`, fields written by name against the engine's defaults -- and a file
//! written from outside it would couple the exporter to engine serialization. A script uses
//! the engine's own API to build the scene, so what it makes is whatever that version of
//! Flax makes, and it is a readable C# file when it needs correcting.
//!
//! Axes and units: the level is in Godot's frame -- right-handed, Y up, metres. Flax is
//! left-handed, Y up, and speaks centimetres. Right-handed Y-up to left-handed Y-up is
//! one axis negated, and Z is the one that puts Godot's forward (-Z) on Flax's forward
//! (+Z), so a translation converts as `(x, y, -z) * 100` and the yaw turns the other way.

use crate::level::{CollisionMesh, Level};

/// One file the Flax writer asks to be written, path relative to the export directory.
#[derive(Debug, Clone)]
pub struct FlaxFile {
    /// Where it goes, relative to the export directory.
    pub path: String,
    /// What is in it.
    pub contents: String,
}

/// How the Flax script is generated.
#[derive(Debug, Clone)]
pub struct FlaxOptions {
    /// The folder the meshes are expected to land in, seen from the project's `Content`
    /// directory, e.g. `Levels/arena`. Only the generated script's own comments and its
    /// asset paths use it.
    pub content_dir: String,
}

impl FlaxOptions {
    /// Options for a level of the given name: content under `Content/<name>`.
    pub fn for_level(name: &str) -> Self {
        FlaxOptions {
            content_dir: name.to_string(),
        }
    }
}

pub fn validate(level: &Level) -> Result<(), String> {
    use crate::level::Value;
    for entity in &level.entities {
        if let Some(value) = entity.properties.get("flax_prefab") {
            if !matches!(value,Value::String(path) if !std::path::Path::new(path).is_absolute()&&path.ends_with(".prefab")&& !path.split('/').any(|part|part==".."))
            {
                return Err(format!(
                    "{}: flax_prefab must name an project Content/*.prefab resource",
                    entity.name
                ));
            }
        }
        if let Some(value) = entity.properties.get("flax_body_type") {
            let Value::String(body) = value else {
                return Err(format!("{}: flax_body_type must be a string", entity.name));
            };
            if !matches!(body.as_str(), "static" | "kinematic" | "dynamic") {
                return Err(format!("{}: invalid flax_body_type {body}", entity.name));
            }
            if body == "dynamic"
                && (entity.properties.contains_key("_heron_animation")
                    || entity
                        .collision
                        .as_ref()
                        .is_some_and(|shape| shape.kind != crate::level::CollisionShape::Convex))
            {
                return Err(format!("{}: dynamic Flax bodies require convex collision and cannot use exported animation",entity.name));
            }
        }
        if let Some(value) = entity.properties.get("flax_mass") {
            let mass = match value {
                Value::Float(value) => *value,
                Value::Int(value) => *value as f64,
                _ => return Err(format!("{}: flax_mass must be numeric", entity.name)),
            };
            if !mass.is_finite() || mass <= 0. {
                return Err(format!("{}: flax_mass must be positive", entity.name));
            }
        }
    }
    Ok(())
}

/// Centimetres to a metre, which is what Flax measures in.
const CENTIMETRES: f64 = 100.0;

/// Builds the C# import script, every placement baked in.
pub fn write(level: &Level, options: &FlaxOptions) -> Vec<FlaxFile> {
    write_with_collision(level, options, &[])
}

/// Builds the importer and includes entity-local geometry for persistent native colliders.
pub fn write_with_collision(
    level: &Level,
    options: &FlaxOptions,
    collision: &[CollisionMesh],
) -> Vec<FlaxFile> {
    write_importer(level, options, collision, false)
}

fn write_importer(
    level: &Level,
    options: &FlaxOptions,
    collision: &[CollisionMesh],
    live: bool,
) -> Vec<FlaxFile> {
    let manifest = super::three::write(level, collision);
    let metadata_root: serde_json::Value =
        serde_json::from_str(&manifest).expect("serialized level");
    let mut entries = String::new();
    for (index, entity) in level.entities.iter().enumerate() {
        let t = &entity.transform;
        let (gx, gy, gz) = (t.translation[0], t.translation[1], t.translation[2]);
        let (fx, fy, fz) = (gx * CENTIMETRES, gy * CENTIMETRES, -gz * CENTIMETRES);
        // Reflect the full YXZ rotation across Z. Components are [pitch, yaw, roll].
        let q = glam::DQuat::from_euler(
            glam::EulerRot::YXZ,
            t.rotation_yxz[1],
            t.rotation_yxz[0],
            t.rotation_yxz[2],
        );
        let (qx, qy, qz, qw) = (-q.x, -q.y, q.z, q.w);
        let [sx, sy, sz] = t.scale;
        let metadata = escape(
            &serde_json::to_string(&metadata_root["entities"][index]).expect("finite level entity"),
        );
        entries.push_str(&format!(
            "        new Placement(\"{name}\", \"{classname}\", {mesh}, \
             new Vector3({fx:.3}f, {fy:.3}f, {fz:.3}f), \
             new Quaternion({qx:.9}f, {qy:.9}f, {qz:.9}f, {qw:.9}f), \
             new Vector3({sx:.9}f, {sy:.9}f, {sz:.9}f), \
             \"{metadata}\"),\n",
            name = escape(&entity.name),
            classname = escape(&entity.classname),
            // `mesh` alone is not "there is something to draw": a trigger volume has a
            // `.glb` too, holding its collision and nothing in the file's scene. What
            // says an entity draws is `surfaces`, one per material it wears.
            mesh = match &entity.mesh {
                Some(path) if !entity.surfaces.is_empty() => {
                    format!("\"{}\"", escape(&asset_path(path, &options.content_dir)))
                }
                _ => "null".to_string(),
            },
        ));
    }

    let class_name = if live {
        "HeronBrushLiveBuilder"
    } else {
        "HeronBrushLevel"
    };
    let placements = if live {
        include_str!("flax_live_placements.cs").to_string()
    } else {
        format!(
            "private static readonly Placement[] Placements =\n        {{\n{entries}        }};"
        )
    };
    let import_skip = if live {
        "if (File.Exists(path + \".flax\")) continue;"
    } else {
        ""
    };
    // Editor live previews remain at the document pose and do not need the runtime companion.
    let animation_attach = if live {
        ""
    } else {
        r#"            foreach (var placement in Placements)
            {
                var properties = JObject.Parse(placement.MetadataJson)["properties"];
                string waterJson = (string)properties?["_heron_water"]?["value"];
                if (!string.IsNullOrEmpty(waterJson)) actors[placement.Name].AddScript<HeronBrushWater>().DataJson=waterJson;
                string animationJson = (string)properties?["_heron_animation"]?["value"];
                if (!string.IsNullOrEmpty(animationJson))
                    actors[placement.Name].AddScript<HeronBrushAnimation>().DataJson = animationJson;
                string triggerJson = (string)properties?["_heron_trigger"]?["value"];
                if (!string.IsNullOrEmpty(triggerJson))
                {
                    var trigger = actors[placement.Name].AddScript<HeronBrushAnimationTrigger>();
                    trigger.DataJson = triggerJson;
                    trigger.LevelRoot = root;
                    var targets = (JArray)JObject.Parse(triggerJson)["targets"];
                    var resolved = new List<Actor>();
                    foreach (string name in targets) if (actors.TryGetValue(name, out var target)) resolved.Add(target);
                    trigger.Targets = resolved.ToArray();
                }
            }
"#
    };
    let content = escape(&options.content_dir);
    let mut script = format!(
        r#"// Generated by Heron Brush. A Flax Engine editor script.
//
// Put the exported folder under the project's Content directory -- Flax imports the
// .glb meshes as it appears -- then put this file in the project's Source/Editor
// module together with HeronBrushAnimation.cs and use Tools > Heron Brush import. It imports visible GLB models at
// 100 cm per metre and builds actors in the open scene. The callback on Build
// lets your integration attach components using complete MetadataJson.
// Reimport replaces only the root created by this menu during this editor session.
// Save the resulting scene. Metadata stays in the adjacent .level.json file.
//
// The meshes are expected under Content/{content}. Change ContentDir if they went
// somewhere else.

using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using Newtonsoft.Json.Linq;
using FlaxEngine;

namespace Game
{{
    /// <summary>Builds the level Heron Brush exported.</summary>
    public static class {class_name}
    {{
        /// <summary>Where the imported meshes live, under Content.</summary>
        public const string ContentDir = "{content}";
        public const string SourceKey = "{source_key}";
        public static Actor LastBuiltRoot {{ get; private set; }}
{bindings_helper}
        public static Dictionary<string,Actor> Rebuild(Action<Actor, Placement, IReadOnlyDictionary<string, Actor>> configure = null)
        {{
            Actor previous=null;
            foreach(var owner in Level.GetScripts<HeronBrushLevelRoot>())if(owner.SourceKey==SourceKey){{previous=owner.Actor;break;}}
            var actors=Build(configure);var root=LastBuiltRoot;
            if(previous!=null&&root!=null){{root.Parent=previous.Parent;root.LocalTransform=previous.LocalTransform;root.IsActive=previous.IsActive;FlaxEngine.Object.DestroyNow(previous);}}
            return actors;
        }}

        public readonly struct Placement
        {{
            public readonly string Name;
            public readonly string Classname;
            public readonly string Mesh;
            public readonly Vector3 Position;
            public readonly Quaternion Orientation;
            public readonly Vector3 Scale;
            // Complete engine-neutral entity, including typed properties, resolved
            // targets, tags, scene mapping and collision. Coordinates remain metres.
            public readonly string MetadataJson;

            public Placement(string name, string classname, string mesh, Vector3 position, Quaternion orientation, Vector3 scale, string metadataJson)
            {{
                Name = name;
                Classname = classname;
                Mesh = mesh;
                Position = position;
                Orientation = orientation;
                Scale = scale;
                MetadataJson = metadataJson;
            }}
        }}

        {placements}

#if FLAX_EDITOR
        public static void ImportModels(bool preserveExistingMaterials = false)
        {{
            var imported = new HashSet<string>();
            foreach (var placement in Placements)
            {{
                if (placement.Mesh == null || !imported.Add(placement.Mesh)) continue;
                var path = Path.Combine(Globals.ProjectContentFolder, placement.Mesh);
                {import_skip}
                var options = FlaxEngine.Tools.ModelTool.Options.Default;
                options.Scale = 100.0f;
                options.ImportMaterials = true;
                options.ImportTextures = true;
                options.RestoreMaterialsOnReimport = preserveExistingMaterials;
                options.SkipExistingMaterialsOnReimport = preserveExistingMaterials;
                if (FlaxEditor.Editor.Import(path + ".glb", path + ".flax", options))
                    throw new InvalidOperationException("Model import failed: " + path);
            }}
        }}
#endif
        private readonly struct CollisionPart
        {{
            public readonly CollisionData Data;
            public readonly bool Sensor;
            public CollisionPart(CollisionData data, bool sensor) {{ Data = data; Sensor = sensor; }}
        }}
        private static CollisionData LoadCollision(JArray coordinates, bool convex)
        {{
            if (coordinates.Count % 3 != 0 || coordinates.Count < (convex ? 12 : 9))
                throw new InvalidOperationException("Invalid collision vertex array.");
            if (!convex && coordinates.Count % 9 != 0) throw new InvalidOperationException("Incomplete collision triangle.");
            var vertices = new Float3[coordinates.Count / 3];
            for (int i = 0; i < vertices.Length; i++)
                vertices[i] = new Float3((float)coordinates[i*3]*100, (float)coordinates[i*3+1]*100, -(float)coordinates[i*3+2]*100);
            // Convex cooking computes the hull from this exact point cloud.
            int[] indices = new int[convex ? (vertices.Length-2)*3 : vertices.Length];
            for (int t = 0; t < indices.Length/3; t++)
            {{
                // Keep the exported triangle order; the OBJ adapter below reverses it.
                indices[t*3] = convex ? 0 : t*3;
                indices[t*3+1] = convex ? t+2 : t*3+1;
                indices[t*3+2] = convex ? t+1 : t*3+2;
            }}
            string key;
            using (var sha = SHA256.Create())
                key = BitConverter.ToString(sha.ComputeHash(Encoding.UTF8.GetBytes("bb-flax-collision-v3:" + convex + ":" + coordinates.ToString(Newtonsoft.Json.Formatting.None)))).Replace("-", "").ToLowerInvariant();
            string directory = Path.Combine(Globals.ProjectContentFolder, ContentDir + "_collision");
            string path = Path.Combine(directory,key,"Collision.flax");
#if FLAX_EDITOR
            if (!File.Exists(path))
            {{
                Directory.CreateDirectory(Path.GetDirectoryName(path));
                string sourcePath = Path.Combine(directory,key+".obj");
                string modelPath = Path.Combine(directory,key+".source.flax");
                // Import-time cooking reads CPU source data, including under the Null renderer.
                // A hidden helper model carries the collision submesh; no actor renders it.
                var obj = new StringBuilder("o COLL_HeronBrush\n");
                foreach (var vertex in vertices)
                    obj.AppendFormat(System.Globalization.CultureInfo.InvariantCulture,"v {{0:R}} {{1:R}} {{2:R}}\n",vertex.X,vertex.Y,-vertex.Z);
                for (int i=0;i<indices.Length;i+=3)
                    obj.AppendFormat("f {{0}} {{1}} {{2}}\n",indices[i]+1,indices[i+2]+1,indices[i+1]+1);
                obj.Append("o SourcePlaceholder\nv 0 0 0\nv 1 0 0\nv 0 1 0\n");
                obj.AppendFormat("f {{0}} {{1}} {{2}}\n",vertices.Length+1,vertices.Length+2,vertices.Length+3);
                File.WriteAllText(sourcePath,obj.ToString());
                var options=FlaxEngine.Tools.ModelTool.Options.Default;
                // Flax applies OBJ centimetre conversion implicitly; our vertices are already cm.
                options.Scale=0.01f;options.SubAssetFolder=key;options.ImportMaterials=false;options.ImportTextures=false;
                options.CollisionMeshesPrefix="COLL_";
                options.CollisionType=convex?CollisionDataType.ConvexMesh:CollisionDataType.TriangleMesh;
                if (FlaxEditor.Editor.Import(sourcePath,modelPath,options)) throw new InvalidOperationException("Collision source import failed");
            }}
#endif
            var data = Content.Load<CollisionData>(path);
            if (data == null || data.IsVirtual || data.WaitForLoaded()) throw new InvalidOperationException("Cannot load persistent collision: " + path);
            return data;
        }}
        private static List<CollisionPart> CollisionFor(Placement placement)
        {{
            var result = new List<CollisionPart>();
            var collision = JObject.Parse(placement.MetadataJson)["collision"] as JObject;
            if (collision == null) return result;
            bool sensor = (bool?)collision["sensor"] ?? false;
            if (collision["hulls"] is JArray hulls && hulls.Count > 0)
                foreach (JArray hull in hulls) result.Add(new CollisionPart(LoadCollision(hull, true), sensor));
            else if (collision["triangles"] is JArray triangles && triangles.Count > 0)
            {{
                if (sensor) throw new InvalidOperationException("A Flax trigger requires convex hulls: " + placement.Name);
                result.Add(new CollisionPart(LoadCollision(triangles, false), false));
            }}
            else throw new InvalidOperationException("Missing collision geometry: " + placement.Name);
            return result;
        }}

        /// <summary>Spawns every actor of the level into the open scene.</summary>
        // Called only after all actors exist, so links can resolve forward references.
        // Supply your own game components here. No classname implies game behaviour.
        public static Dictionary<string, Actor> Build(Action<Actor, Placement, IReadOnlyDictionary<string, Actor>> configure = null, bool createCollisions = true, Func<Placement, bool> includeCollision = null)
        {{
            // Validate dependencies before changing the scene.
            var models = new Dictionary<string, Model>();
            var metadata = new Dictionary<string,JObject>();
            var prefabs = new Dictionary<string,Prefab>();
            foreach (var placement in Placements)
            {{
                var data=JObject.Parse(placement.MetadataJson);metadata.Add(placement.Name,data);
                var prefabPath=(string)Property(data,"flax_prefab");
                if(prefabPath!=null)
                {{
                    var prefab=Content.Load<Prefab>(Path.Combine(Globals.ProjectContentFolder,prefabPath));
                    if(prefab==null)throw new InvalidOperationException("Missing project prefab " + prefabPath);
                    prefabs.Add(placement.Name,prefab);
                }}
                if(data["properties"] is JObject properties)
                    foreach(var property in properties.Properties())
                        if(property.Name.StartsWith("flax_script_"))ScriptType((string)property.Value["value"]);
                if (placement.Mesh == null || models.ContainsKey(placement.Mesh)) continue;
                var path = Path.Combine(Globals.ProjectContentFolder, placement.Mesh + ".flax");
                var model = Content.Load<Model>(path);
                if (model == null) throw new InvalidOperationException("Missing imported model: " + path);
                models.Add(placement.Mesh, model);
            }}
            var collisions = new Dictionary<string, List<CollisionPart>>();
            if (createCollisions)
                foreach (var placement in Placements)
                    if (includeCollision == null || includeCollision(placement))
                        collisions.Add(placement.Name, CollisionFor(placement));
            if (Level.ScenesCount == 0 || Level.GetScene(0) == null) throw new InvalidOperationException("Open a scene before importing the level.");
            // The root is active from the start: Flax 1.12 builds colliders spawned under an
            // inactive parent so that rays hit them but rigid bodies and triggers do not.
            var root = new EmptyActor {{ Name = "{level}" }};
            if (Level.SpawnActor(root)) throw new InvalidOperationException("Could not spawn level root.");

            root.AddScript<HeronBrushLevelRoot>().SourceKey = SourceKey;
            var actors = new Dictionary<string, Actor>();
            try
            {{
            foreach (var placement in Placements)
            {{
                Actor actor;
                var data = metadata[placement.Name];
                var bodyType = (string)Property(data,"flax_body_type");
                if (prefabs.TryGetValue(placement.Name,out var prefab))
                {{
                    actor=PrefabManager.SpawnPrefab(prefab,root);
                    if(actor==null)throw new InvalidOperationException("Cannot spawn prefab for " + placement.Name);
                }}
                else if (bodyType == "dynamic" || bodyType == "kinematic")
                {{
                    actor = new RigidBody {{ IsKinematic = bodyType == "kinematic", EnableGravity = bodyType == "dynamic" }};
                }}
                else if (placement.Mesh == null)
                {{
                    // A point entity: an empty actor carrying the classname, for a
                    // script of yours to find and turn into whatever it means.
                    actor = new EmptyActor();
                }}
                else
                {{
                    var staticModel = new StaticModel {{ Model = models[placement.Mesh] }};
                    actor = staticModel;
                }}

                actor.Name = placement.Name;
                actor.LocalPosition = placement.Position;
                actor.LocalOrientation = placement.Orientation;
                actor.LocalScale = placement.Scale;
                if (actor.Parent != root && Level.SpawnActor(actor, root)) throw new InvalidOperationException("Could not spawn " + placement.Name);
                if(actor is RigidBody body)
                {{
                    if(bodyType!=null){{body.IsKinematic=bodyType!="dynamic";body.EnableGravity=bodyType=="dynamic";}}
                    var mass=Property(data,"flax_mass");if(mass!=null){{body.OverrideMass=true;body.Mass=(float)mass;}}
                }}
                else if(bodyType=="dynamic"||bodyType=="kinematic"||Property(data,"flax_mass")!=null)throw new InvalidOperationException("Prefab root must be a RigidBody for " + placement.Name);
                if(placement.Mesh!=null && !(actor is StaticModel))
                {{
                    var visual=new StaticModel{{Name="Generated visual",Model=models[placement.Mesh]}};
                    if(Level.SpawnActor(visual,actor))throw new InvalidOperationException("Cannot spawn visual for " + placement.Name);
                    visual.LocalTransform=Transform.Identity;
                }}
                if(data["properties"] is JObject properties)
                    foreach(var property in properties.Properties())
                        if(property.Name.StartsWith("flax_script_"))actor.AddScript(ScriptType((string)property.Value["value"]));
                actors.Add(placement.Name, actor);
                if (collisions.TryGetValue(placement.Name, out var parts))
                    for (int i = 0; i < parts.Count; i++)
                    {{
                        var collider = new MeshCollider {{ Name = "Collision " + i, CollisionData = parts[i].Data, IsTrigger = parts[i].Sensor }};
                        if (Level.SpawnActor(collider, actor)) throw new InvalidOperationException("Cannot spawn collision for " + placement.Name);
                        collider.LocalPosition = Vector3.Zero;
                        collider.LocalOrientation = Quaternion.Identity;
                        collider.LocalScale = Vector3.One;
                    }}
            }}

{animation_attach}
            foreach(var placement in Placements)
                ConfigureProperties(actors[placement.Name], metadata[placement.Name], actors);
            if (configure != null)
                foreach (var placement in Placements)
                    configure(actors[placement.Name], placement, actors);
            LastBuiltRoot = root;
            Debug.Log("Heron Brush: built " + actors.Count + " actors");
            return actors;
            }}
            catch
            {{
                FlaxEngine.Object.Destroy(root);
                throw;
            }}
        }}
    }}
#if FLAX_EDITOR
    public sealed class HeronBrushImportMenu : FlaxEditor.EditorPlugin
    {{
        private FlaxEditor.GUI.ContextMenu.ContextMenuButton _button;
        public override void InitializeEditor()
        {{
            base.InitializeEditor();
            if (Editor.UI.MainMenu != null)
                _button = Editor.UI.AddMenuButton("Tools", "Heron Brush import", Import);
        }}
        private void Import()
        {{
            HeronBrushLevel.ImportModels();
            HeronBrushLevel.Rebuild();
        }}
        public override void DeinitializeEditor()
        {{
            _button?.Dispose();
            base.DeinitializeEditor();
        }}
    }}
#endif
}}
"#,
        level = escape(&level.name),
        source_key = namespace(level, options),
        bindings_helper = include_str!("flax_bindings.cs"),
    );

    if live {
        // The standalone import menu is replaced by the live editor plugin.
        let start = script
            .find("#if FLAX_EDITOR\n    public sealed class HeronBrushImportMenu")
            .expect("import menu section");
        script.truncate(start);
        script.push_str("}\n");
    }
    let mut files = vec![
        FlaxFile {
            path: format!("{class_name}.cs"),
            contents: script,
        },
        FlaxFile {
            path: format!("{}.level.json", level.name),
            contents: manifest,
        },
        FlaxFile {
            path: "HeronBrushAnimation.cs".into(),
            contents: include_str!("flax_animation.cs").into(),
        },
        FlaxFile {
            path: "HeronBrushMotionPlayer.cs".into(),
            contents: include_str!("flax_player.cs").into(),
        },
        FlaxFile {
            path: "HeronBrushWater.cs".into(),
            contents: include_str!("flax_water.cs").into(),
        },
        FlaxFile {
            path: "HeronBrushLevelRoot.cs".into(),
            contents: include_str!("flax_level_root.cs").into(),
        },
    ];
    if !live {
        let namespace = namespace(level, options);
        for file in &mut files {
            if file.path.ends_with(".cs") {
                file.contents = file
                    .contents
                    .replace("namespace Game", &format!("namespace {namespace}"));
            }
        }
    }
    files
}

pub fn namespace(level: &Level, options: &FlaxOptions) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in format!("{}\n{}", options.content_dir.replace('\\', "/"), level.name).bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("Game.HeronLevel_{hash:016x}")
}

/// Turns `arena/worldspawn.glb` into the path Flax loads an imported asset by:
/// `<content dir>/worldspawn`, with the level's own folder and the extension off.
fn asset_path(mesh: &str, content_dir: &str) -> String {
    let file = mesh.rsplit('/').next().unwrap_or(mesh);
    let stem = file.strip_suffix(".glb").unwrap_or(file);
    if content_dir.is_empty() {
        stem.to_string()
    } else {
        format!("{content_dir}/{stem}")
    }
}

/// Escapes a string for a C# literal.
fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// A fixed importer installed once; live updates contain only meshes and JSON.
pub fn live_importer() -> String {
    let level = Level {
        format: crate::level::FORMAT_VERSION,
        generator: "Heron Brush".into(),
        name: "Heron Brush Live".into(),
        scale: 32.0,
        textures: vec![],
        entities: vec![],
    };
    write_importer(
        &level,
        &FlaxOptions::for_level("HeronBrushLive/preview"),
        &[],
        true,
    )
    .remove(0)
    .contents
}

/// Flax-native placement values plus complete neutral metadata/collision geometry.
pub fn live_snapshot(level: &Level, collision: &[CollisionMesh]) -> String {
    let metadata: serde_json::Value =
        serde_json::from_str(&super::three::write(level, collision)).expect("serialized level");
    let placements: Vec<_> = level.entities.iter().enumerate().map(|(i,e)| {
        let t = &e.transform;
        let q = glam::DQuat::from_euler(glam::EulerRot::YXZ, t.rotation_yxz[1], t.rotation_yxz[0], t.rotation_yxz[2]);
        serde_json::json!({ "name": e.name, "classname": e.classname,
            "mesh": e.mesh.as_ref().filter(|_| !e.surfaces.is_empty()).map(|p| asset_path(p, "HeronBrushLive/preview")),
            "position": [t.translation[0]*100.0, t.translation[1]*100.0, -t.translation[2]*100.0],
            "orientation": [-q.x, -q.y, q.z, q.w], "scale": t.scale, "metadata": metadata["entities"][i] })
    }).collect();
    serde_json::json!({"version":1, "placements":placements}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{Entity, Level, Surface, Transform};

    fn level() -> Level {
        Level {
            format: 2,
            generator: "test".to_string(),
            name: "arena".to_string(),
            scale: 32.0,
            textures: Vec::new(),
            entities: vec![
                Entity {
                    name: "worldspawn".to_string(),
                    classname: "worldspawn".to_string(),
                    mesh: Some("arena/worldspawn.glb".to_string()),
                    // A surface is what says the entity draws; without one it is a
                    // volume whose `.glb` holds only collision.
                    surfaces: vec![Surface {
                        primitive: 0,
                        texture: "stone".to_string(),
                    }],
                    transform: Transform::default(),
                    ..Entity::default()
                },
                Entity {
                    name: "info_player_start".to_string(),
                    classname: "info_player_start".to_string(),
                    mesh: None,
                    transform: Transform {
                        translation: [1.0, 2.0, 3.0],
                        rotation_yxz: [0.0, std::f64::consts::FRAC_PI_2, 0.0],
                        scale: [1.0, 1.0, 1.0],
                    },
                    ..Entity::default()
                },
            ],
        }
    }

    #[test]
    fn a_translation_is_converted_to_flaxs_frame_and_centimetres() {
        let files = write(&level(), &FlaxOptions::for_level("arena"));
        let script = &files[0].contents;
        // Metres to centimetres, and Z negated for the handedness.
        assert!(
            script.contains("new Vector3(100.000f, 200.000f, -300.000f)"),
            "{script}"
        );
        // A quarter turn one way in Godot's frame is a quarter turn the other in Flax's.
        assert!(script.contains("-0.707106781f"), "{script}");
    }

    #[test]
    fn a_mesh_is_named_the_way_flax_loads_an_imported_asset() {
        let files = write(&level(), &FlaxOptions::for_level("arena"));
        let script = &files[0].contents;
        assert!(script.contains("\"arena/worldspawn\""), "{script}");
        assert!(
            !script.contains("\"arena/worldspawn.glb\""),
            "placement must store an asset stem:\n{script}"
        );
    }

    #[test]
    fn a_volume_that_draws_nothing_is_not_given_a_model() {
        // A trigger carries a `.glb` holding its collision, and nothing in that file's
        // scene. Reading `mesh` alone and making a StaticModel of it would draw the
        // trigger.
        let mut level = level();
        level.entities.push(Entity {
            name: "trigger_multiple".to_string(),
            classname: "trigger_multiple".to_string(),
            mesh: Some("arena/trigger_multiple.glb".to_string()),
            surfaces: Vec::new(),
            transform: Transform::default(),
            ..Entity::default()
        });

        let files = write(&level, &FlaxOptions::for_level("arena"));
        let script = &files[0].contents;
        assert!(
            script.contains("\"trigger_multiple\", \"trigger_multiple\", null"),
            "{script}"
        );
    }

    #[test]
    fn colliders_are_spawned_under_an_active_root() {
        // Under an inactive root, Flax's colliders answer raycasts but take no part
        // in the simulation, so dynamic bodies fall through them.
        let files = write(&level(), &FlaxOptions::for_level("arena"));
        let script = &files[0].contents;
        assert!(
            script.contains("var root = new EmptyActor { Name = \"arena\" };"),
            "{script}"
        );
        assert!(!script.contains("IsActive = false"), "{script}");
    }

    #[test]
    fn an_entity_with_nothing_to_draw_still_gets_an_actor() {
        let files = write(&level(), &FlaxOptions::for_level("arena"));
        let script = &files[0].contents;
        assert!(
            script.contains("\"info_player_start\", \"info_player_start\", null"),
            "{script}"
        );
    }
    #[test]
    fn metadata_and_all_transform_components_survive() {
        let mut input = level();
        let e = &mut input.entities[1];
        e.transform.rotation_yxz = [0.3, -0.7, 0.4];
        e.transform.scale = [2.0, 0.5, 3.0];
        e.properties
            .insert("custom_speed".into(), crate::level::Value::Float(2.75));
        e.targets.push("worldspawn".into());
        e.tags.push("custom_tag".into());
        let files = write(&input, &FlaxOptions::for_level("arena"));
        let json: serde_json::Value = serde_json::from_str(&files[1].contents).unwrap();
        assert_eq!(
            json["entities"][1]["properties"]["custom_speed"]["value"],
            2.75
        );
        assert_eq!(json["entities"][1]["targets"][0], "worldspawn");
        assert!(files[0]
            .contents
            .contains("new Vector3(2.000000000f, 0.500000000f, 3.000000000f)"));
        assert!(files[0]
            .contents
            .contains("configure(actors[placement.Name], placement, actors)"));
        assert!(files[0]
            .contents
            .contains("actor.LocalOrientation = placement.Orientation"));
    }
}
