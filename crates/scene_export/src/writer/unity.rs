//! Writes a Unity import script beside the meshes.
//!
//! Unity reads `.obj` out of the box and glTF not at all, so the Unity target writes
//! each solid as an `.obj` (`MeshFormat::Obj`) and generates one C# Editor script with
//! the whole placement baked in: a menu item under **Heron Brush → Build Level** walks
//! the baked entries, instantiates each mesh asset, positions it, and gives it a
//! `MeshCollider` where the map says it collides. Baking the entries as code sidesteps
//! JSON parsing in C# entirely -- `JsonUtility` cannot read a property map -- and makes
//! the import a one-click affair with no packages to install.
//!
//! Axes: the level is in Godot's frame (right-handed, Y up, metres). Unity is
//! left-handed Y up, and its `.obj` importer negates X to convert, so the script
//! negates X on every translation to keep the world and its meshes in step, and flips
//! the yaw and roll signs for the same reason.

use crate::level::{CollisionMesh, Level};

/// Shared implementation embedded into both Unity importer classes.
pub const COLLISION_HELPER: &str = include_str!("unity_collision.cs");

/// One file the Unity writer asks to be written, path relative to the export directory.
#[derive(Debug, Clone)]
pub struct UnityFile {
    pub path: String,
    pub contents: String,
}

/// How the Unity script is generated.
#[derive(Debug, Clone)]
pub struct UnityOptions {
    /// Assets path the exported folder is expected to land under, e.g.
    /// `Assets/Levels`. Also forms the stable per-export namespace; the script
    /// finds the meshes relative to its own source asset.
    pub assets_hint: String,
    /// Generated materials whose normal map the export writes beside their albedo
    /// as `<name>_normal.png`, in the OpenGL convention Unity uses.
    pub normal_maps: Vec<String>,
    pub sky: Option<super::sky::Sky>,
}

impl Default for UnityOptions {
    fn default() -> Self {
        UnityOptions {
            assets_hint: "Assets/Levels".to_string(),
            normal_maps: Vec::new(),
            sky: None,
        }
    }
}

pub fn validate(level: &Level) -> Result<(), String> {
    use crate::level::Value;
    for entity in &level.entities {
        if let Some(value) = entity.properties.get("unity_prefab") {
            if !matches!(value,Value::String(path) if path.starts_with("Assets/")&&path.ends_with(".prefab"))
            {
                return Err(format!(
                    "{}: unity_prefab must name an Assets/*.prefab resource",
                    entity.name
                ));
            }
        }
        if let Some(path) = prefab_path(entity) {
            if !path.starts_with("Assets/") {
                return Err(format!(
                    "{}: Unity prefab scene must name an Assets/*.prefab resource",
                    entity.name
                ));
            }
        }
        if let Some(value) = entity.properties.get("unity_body_type") {
            let Value::String(body) = value else {
                return Err(format!("{}: unity_body_type must be a string", entity.name));
            };
            if !matches!(body.as_str(), "static" | "kinematic" | "dynamic") {
                return Err(format!("{}: invalid unity_body_type {body}", entity.name));
            }
            if (body != "kinematic" && entity.properties.contains_key("_heron_animation"))
                || (body == "dynamic"
                    && entity
                        .collision
                        .as_ref()
                        .is_some_and(|shape| shape.kind != crate::level::CollisionShape::Convex))
            {
                return Err(format!(
                    "{}: dynamic Unity bodies require convex collision and cannot use exported animation",
                    entity.name
                ));
            }
        }
        if let Some(value) = entity.properties.get("unity_mass") {
            let mass = match value {
                Value::Float(value) => *value,
                Value::Int(value) => *value as f64,
                _ => return Err(format!("{}: unity_mass must be numeric", entity.name)),
            };
            if !mass.is_finite() || mass <= 0. {
                return Err(format!("{}: unity_mass must be positive", entity.name));
            }
        }
    }
    Ok(())
}

fn prefab_path(entity: &crate::level::Entity) -> Option<&str> {
    entity
        .properties
        .get("unity_prefab")
        .and_then(|value| match value {
            crate::level::Value::String(value) => Some(value.as_str()),
            _ => None,
        })
        .or_else(|| {
            entity
                .scene
                .as_deref()
                .filter(|path| path.ends_with(".prefab"))
        })
        .map(|path| path.strip_prefix("res://").unwrap_or(path))
}

/// Builds the generated files: the Editor script, with every placement baked in.
pub fn write(level: &Level, collision: &[CollisionMesh], options: &UnityOptions) -> Vec<UnityFile> {
    write_with_materials(level, collision, options, &indexmap::IndexMap::new())
}

/// Preserve generated glTF material maps through Unity's OBJ geometry path.
pub fn write_with_materials(
    level: &Level,
    collision: &[CollisionMesh],
    options: &UnityOptions,
    materials: &indexmap::IndexMap<String, crate::materials::GeneratedMaterial>,
) -> Vec<UnityFile> {
    let mut entries = String::new();
    let mut setup = String::new();
    let mut property_setup = String::from(
        "        var namedNodes = new Dictionary<string,GameObject>();\n        foreach(var node in nodes) namedNodes.Add(node.name,node);\n",
    );
    for (index, entity) in level.entities.iter().enumerate() {
        let metadata = serde_json::to_string(entity).expect("entity JSON");
        for (name, value) in &entity.properties {
            property_setup.push_str(&format!(
                "        SetProperty(nodes[{index}], \"{}\", {}, namedNodes);\n",
                escape(name),
                property_literal(value)
            ));
        }
        setup.push_str(&format!(
            "        nodes[{index}].AddComponent<HeronBrushEntity>().entityJson = \"{}\";\n",
            escape(&metadata)
        ));
        if let Some(crate::level::Value::String(data)) = entity.properties.get("_heron_water") {
            setup.push_str(&format!(
                "        nodes[{index}].AddComponent<HeronBrushWater>().dataJson=\"{}\";\n",
                escape(data)
            ));
        }
        if let Some(light) = super::common_entities::light(entity) {
            let [r, g, b, a] = light.color;
            setup.push_str(&format!(
                "        var light{index} = nodes[{index}].AddComponent<Light>();\n        light{index}.type = LightType.{};\n        light{index}.color = new Color({r:.6}f,{g:.6}f,{b:.6}f,{a:.6}f);\n        light{index}.intensity = {:.6}f;\n        light{index}.range = {:.6}f;\n        light{index}.spotAngle = {:.6}f;\n        light{index}.shadows = LightShadows.{};\n        light{index}.transform.Rotate(0f,180f,0f,Space.Self);\n",
                light.kind, light.energy, light.range, light.half_angle*2., if light.shadows { "Soft" } else { "None" }));
        }
        if let Some(shapes) = collision.get(index) {
            let sensor = entity.collision.as_ref().is_some_and(|c| c.sensor);
            let mut add = |points: Vec<f32>, convex: bool| {
                let points = points
                    .iter()
                    .map(|v| format!("{v:.9}f"))
                    .collect::<Vec<_>>()
                    .join(",");
                setup.push_str(&format!("        AddCollision(nodes[{index}], new Shape {{ convex = {convex}, points = new float[] {{ {points} }} }}, {sensor}, ToAssetPath(Path.Combine(here, \"Collision\")));\n"));
            };
            if !shapes.triangles.is_empty() {
                add(shapes.triangles.iter().flatten().copied().collect(), false);
            }
            for hull in &shapes.hulls {
                if hull.len() >= 4 {
                    add(hull.iter().flatten().copied().collect(), true);
                }
            }
        }
        if let Some(crate::level::Value::String(data)) = entity.properties.get("_heron_animation") {
            setup.push_str(&format!("        var animation{index}=nodes[{index}].AddComponent<HeronBrushAnimation>();\n        animation{index}.dataJson=\"{}\";\n        var body{index}=nodes[{index}].GetComponent<Rigidbody>(); if(body{index}==null)body{index}=nodes[{index}].AddComponent<Rigidbody>(); body{index}.isKinematic=true; body{index}.useGravity=false;\n",escape(data)));
        }
        if let Some(crate::level::Value::String(body)) = entity.properties.get("unity_body_type") {
            if body == "static" {
                setup.push_str(&format!("        var staticBody{index}=nodes[{index}].GetComponent<Rigidbody>(); if(staticBody{index}!=null)Object.DestroyImmediate(staticBody{index});\n"));
            } else {
                setup.push_str(&format!("        var configuredBody{index}=nodes[{index}].GetComponent<Rigidbody>(); if(configuredBody{index}==null)configuredBody{index}=nodes[{index}].AddComponent<Rigidbody>(); configuredBody{index}.isKinematic={}; configuredBody{index}.useGravity={};\n",body=="kinematic",body=="dynamic"));
            }
        }
        if let Some(mass) = entity.properties.get("unity_mass") {
            setup.push_str(&format!("        var massBody{index}=nodes[{index}].GetComponent<Rigidbody>(); if(massBody{index}==null)throw new System.InvalidOperationException(\"{}: unity_mass needs a Rigidbody\"); massBody{index}.mass=System.Convert.ToSingle({});\n",escape(&entity.name),property_literal(mass)));
        }
        let t = &entity.transform;
        let [rx, ry, rz] = unity_euler_degrees(t.rotation_yxz);
        let [sx, sy, sz] = t.scale;
        let sensor = entity
            .collision
            .as_ref()
            .is_some_and(|collision| collision.sensor);
        let has_collision = entity.collision.is_some()
            || collision.get(index).is_some_and(|c| {
                !c.triangles.is_empty() || !c.boxes.is_empty() || !c.hulls.is_empty()
            });
        // Unity's .obj importer negates X; the placements follow suit, and the
        // yaw and roll flip sign with the handedness.
        entries.push_str(&format!(
            "            new Entry(\"{name}\", \"{classname}\", {mesh}, \
             new Vector3({x:.6}f, {y:.6}f, {z:.6}f), \
             new Vector3({rx:.6}f, {ry:.6}f, {rz:.6}f), \
             new Vector3({sx:.6}f, {sy:.6}f, {sz:.6}f), \
             {collides}, {sensor}, {prefab}),\n",
            name = escape(&entity.name),
            classname = escape(&entity.classname),
            mesh = match &entity.mesh {
                Some(path) => format!("\"{}\"", escape(path)),
                None => "null".to_string(),
            },
            x = -t.translation[0],
            y = t.translation[1],
            z = t.translation[2],
            collides = has_collision,
            sensor = sensor,
            prefab = prefab_path(entity)
                .map(|path| format!("\"{}\"", escape(path)))
                .unwrap_or_else(|| "null".into()),
        ));
    }

    for (index, entity) in level.entities.iter().enumerate() {
        if let Some(crate::level::Value::String(data)) = entity.properties.get("_heron_trigger") {
            let targets = entity
                .targets
                .iter()
                .filter_map(|name| {
                    level.entities.iter().position(|e| {
                        e.name == *name && e.properties.contains_key("_heron_animation")
                    })
                })
                .map(|i| format!("nodes[{i}].GetComponent<HeronBrushAnimation>()"))
                .collect::<Vec<_>>()
                .join(",");
            setup.push_str(&format!("        var trigger{index}=nodes[{index}].AddComponent<HeronBrushAnimationTrigger>(); trigger{index}.dataJson=\"{}\"; trigger{index}.targets=new HeronBrushAnimation[]{{{targets}}};\n",escape(data)));
        }
    }
    let mut surface_setup = String::new();
    for texture in &level.textures {
        if let Some([roughness, metallic, emission]) = texture.surface {
            surface_setup.push_str(&format!(
                "        ApplySurface(root, here, \"{}\", {:.6}f, {:.6}f, {:.6}f);\n",
                escape(&texture.name),
                roughness,
                metallic,
                emission
            ));
        }
    }
    for name in &options.normal_maps {
        surface_setup.push_str(&format!(
            "        ApplyNormal(root, here, \"{}\", \"{}_paint\");\n",
            escape(name),
            escape(&level.name)
        ));
    }
    // Generated paint is drawn once across its face; repeating it would blend the
    // far edge into the near one.
    let clamped = materials
        .iter()
        .filter(|(_, material)| !material.repeat && material.pbr.is_none())
        .map(|(name, _)| format!("\"{}\"", escape(name)))
        .collect::<Vec<_>>();
    if !clamped.is_empty() {
        surface_setup.push_str(&format!(
            "        ClampPaint(here, \"{}_paint\", new[] {{ {} }});\n",
            escape(&level.name),
            clamped.join(", ")
        ));
    }
    for (name, material) in materials {
        let Some(pbr) = &material.pbr else { continue };
        let has = |slot| pbr.images.iter().any(|image| image.slot == slot);
        let [r, g, b] = pbr.emissive_factor;
        surface_setup.push_str(&format!(
            "        ApplyPbr(root, here, \"{}\", \"{}_paint\", {:.6}f, {:.6}f, new Color({r:.6}f,{g:.6}f,{b:.6}f,1f), {:.6}f, {:.6}f, {}, {}, {}, {}, {}, {:.6}f, {});\n",
            escape(name), escape(&level.name), pbr.roughness, pbr.metallic, pbr.normal_scale, pbr.occlusion_strength,
            has("mr"), has("normal"), has("ao"), has("emissive"),
            match pbr.alpha_mode {crate::materials::AlphaMode::Opaque=>0,crate::materials::AlphaMode::Mask=>1,crate::materials::AlphaMode::Blend=>2}, pbr.alpha_cutoff, pbr.double_sided));
    }
    let environment_helper = options
        .sky
        .as_ref()
        .map(|sky| {
            let [x, y, z] = sky.to_sun();
            let [r, g, b] = sky.sun_color;
            let (fog, density) = sky.fog.unwrap_or(([0.; 3], 0.));
            include_str!("unity_environment.cs")
                .replace("__LEVEL__", &escape(&level.name))
                .replace("__EXPOSURE__", &format!("{:.9}", sky.exposure * 1.8))
                .replace("__SUN_COLOR__", &format!("{r:.9}f,{g:.9}f,{b:.9}f"))
                .replace(
                    "__SUN_DIRECTION__",
                    &format!("{x:.9}f,{:.9}f,{:.9}f", -y, -z),
                )
                .replace(
                    "__SUN_INTENSITY__",
                    &format!("{:.9}", sky.sun_intensity / std::f32::consts::PI),
                )
                .replace("__AMBIENT__", &format!("{:.9}", sky.ambient_intensity))
                .replace(
                    "__FOG_ENABLED__",
                    if sky.fog.is_some() { "true" } else { "false" },
                )
                .replace(
                    "__FOG_COLOR__",
                    &format!("{:.9}f,{:.9}f,{:.9}f", fog[0], fog[1], fog[2]),
                )
                .replace("__FOG_DENSITY__", &format!("{density:.9}"))
        })
        .unwrap_or_default();
    let environment_setup = if options.sky.is_some() {
        "        ApplyEnvironment(root, here);"
    } else {
        ""
    };
    let script = format!(
        r#"// Generated by Heron Brush. Drop the exported folder under {hint} and run
// the menu item below; re-running rebuilds the level root from scratch.
#if UNITY_EDITOR
using System.IO;
using System.Collections.Generic;
using UnityEditor;
using UnityEngine;

public static class HeronBrushLevelImporter
{{
    private class Entry
    {{
        public readonly string name;
        public readonly string classname;
        public readonly string mesh;
        public readonly string prefab;
        public readonly Vector3 position;
        public readonly Vector3 eulerDegrees;
        public readonly Vector3 scale;
        public readonly bool collides;
        public readonly bool sensor;

        public Entry(string name, string classname, string mesh,
                     Vector3 position, Vector3 eulerDegrees, Vector3 scale, bool collides, bool sensor, string prefab)
        {{
            this.name = name;
            this.classname = classname;
            this.mesh = mesh;
            this.prefab = prefab;
            this.position = position;
            this.eulerDegrees = eulerDegrees;
            this.scale = scale;
            this.collides = collides;
            this.sensor = sensor;
        }}
    }}

    private static readonly Entry[] Entries =
    {{
{entries}    }};

    [MenuItem("Heron Brush/Build Level: {name}")]
    public static void Build()
    {{
        // The meshes sit beside this script; asset paths are resolved from it.
        var scriptPath = ScriptPath();
        var here = Directory.GetParent(scriptPath).Parent.FullName;
        var sourceGuid = AssetDatabase.AssetPathToGUID(ToAssetPath(scriptPath));
        GameObject previous = null;
        foreach (var owner in Resources.FindObjectsOfTypeAll<HeronBrushLevelRoot>())
            if (!EditorUtility.IsPersistent(owner) && owner.gameObject.scene.IsValid() && owner.sourceGuid == sourceGuid)
            {{ previous = owner.gameObject; break; }}
        // Validate dependencies before changing a scene containing the old level.
        foreach (var entry in Entries)
        {{
            if (entry.mesh != null && AssetDatabase.LoadAssetAtPath<GameObject>(ToAssetPath(Path.Combine(here, entry.mesh))) == null)
                throw new System.InvalidOperationException("Heron Brush: missing mesh " + entry.mesh);
            if (entry.prefab != null && AssetDatabase.LoadAssetAtPath<GameObject>(entry.prefab) == null)
                throw new System.InvalidOperationException("Heron Brush: missing prefab " + entry.prefab);
        }}
        var root = new GameObject("{name}");
        root.SetActive(false);
        root.AddComponent<HeronBrushLevelRoot>().sourceGuid = sourceGuid;
        try {{

        var nodes = new List<GameObject>();
        foreach (var entry in Entries)
        {{
            var node = entry.prefab == null ? new GameObject(entry.name)
                : (GameObject)PrefabUtility.InstantiatePrefab(AssetDatabase.LoadAssetAtPath<GameObject>(entry.prefab),root.transform);
            node.name = entry.name;
            nodes.Add(node);
            node.transform.SetParent(root.transform, false);
            node.transform.localPosition = entry.position;
            node.transform.localEulerAngles = entry.eulerDegrees;
            node.transform.localScale = entry.scale;

            if (entry.mesh != null)
            {{
                var assetPath = ToAssetPath(Path.Combine(here, entry.mesh));
                var meshAsset = AssetDatabase.LoadAssetAtPath<GameObject>(assetPath);
                if (meshAsset != null)
                {{
                    var placed = (GameObject)Object.Instantiate(meshAsset, node.transform);
                    placed.transform.localPosition = Vector3.zero;
                    placed.transform.localEulerAngles = Vector3.zero;
                    placed.transform.localScale = Vector3.one;

                }}
                else
                {{
                    Debug.LogWarning($"Heron Brush: no mesh asset at {{assetPath}}");
                }}
            }}
        }}
{setup}
{property_setup}
{surface_setup}
{environment_setup}
        AssetDatabase.SaveAssets();
        if (previous != null)
        {{
            root.transform.SetParent(previous.transform.parent, false);
            root.transform.localPosition = previous.transform.localPosition;
            root.transform.localRotation = previous.transform.localRotation;
            root.transform.localScale = previous.transform.localScale;
        }}
        root.SetActive(previous == null || previous.activeSelf);
        if (previous != null) Object.DestroyImmediate(previous);
        Debug.Log($"Heron Brush: built {{Entries.Length}} entities under '{name}'");
        }} catch {{ Object.DestroyImmediate(root); throw; }}
    }}

    private static void ApplySurface(GameObject root, string here, string name, float roughness, float metallic, float emission)
    {{
        Material replacement = null;
        foreach (var renderer in root.GetComponentsInChildren<Renderer>(true))
        {{
            var slots = renderer.sharedMaterials;
            for (int i = 0; i < slots.Length; i++)
            {{
                var source = slots[i];
                if (source == null || source.name != name) continue;
                if (replacement == null)
                {{
                    var directory = Path.Combine(here, "Materials");
                    Directory.CreateDirectory(directory);
                    AssetDatabase.Refresh();
                    var path = ToAssetPath(Path.Combine(directory, name + ".mat"));
                    replacement = AssetDatabase.LoadAssetAtPath<Material>(path);
                    bool create = replacement == null;
                    if (create) replacement = new Material(source);
                    var shader = UnityEngine.Rendering.GraphicsSettings.currentRenderPipeline == null
                        ? Shader.Find("Standard") : Shader.Find("Universal Render Pipeline/Lit");
                    if (shader != null) replacement.shader = shader;
                    replacement.name = name;
                    var albedo = source.mainTexture;
                    replacement.mainTexture = albedo;
                    replacement.SetFloat("_Metallic", metallic);
                    if (replacement.HasProperty("_Smoothness")) replacement.SetFloat("_Smoothness", 1f - roughness);
                    if (replacement.HasProperty("_Glossiness")) replacement.SetFloat("_Glossiness", 1f - roughness);
                    replacement.SetTexture("_EmissionMap", albedo);
                    replacement.SetColor("_EmissionColor", Color.white * emission);
                    // The Standard editor recomputes _EMISSION from this flag when it
                    // validates the material; the OBJ material's EmissiveIsBlack
                    // would switch the glow off.
                    replacement.globalIlluminationFlags = emission > 0f
                        ? MaterialGlobalIlluminationFlags.BakedEmissive : MaterialGlobalIlluminationFlags.EmissiveIsBlack;
                    if (emission > 0f) replacement.EnableKeyword("_EMISSION");
                    else replacement.DisableKeyword("_EMISSION");
                    if (create) AssetDatabase.CreateAsset(replacement, path);
                    else EditorUtility.SetDirty(replacement);
                }}
                slots[i] = replacement;
            }}
            renderer.sharedMaterials = slots;
        }}
    }}

    [System.Serializable] private class Shape {{ public bool convex; public float[] points; }}
{collision_helper}
{bindings_helper}
{pbr_helper}
{environment_helper}
    private static string ScriptPath()
    {{
        var guids = AssetDatabase.FindAssets("HeronBrushLevelImporter t:MonoScript");
        foreach (var guid in guids)
        {{
            var path = AssetDatabase.GUIDToAssetPath(guid);
            var script = AssetDatabase.LoadAssetAtPath<MonoScript>(path);
            if (script != null && script.text.Contains("namespace {namespace}")) return Path.GetFullPath(path);
        }}
        throw new System.InvalidOperationException("Heron Brush: importer source asset missing");
    }}

    private static string ToAssetPath(string absolute)
    {{
        var full = Path.GetFullPath(absolute).Replace('\\', '/');
        var project = Path.GetFullPath(".").Replace('\\', '/');
        return full.StartsWith(project) ? full.Substring(project.Length + 1) : full;
    }}
}}
#endif
"#,
        collision_helper = COLLISION_HELPER,
        bindings_helper = include_str!("unity_bindings.cs"),
        pbr_helper = include_str!("unity_pbr.cs"),
        hint = options.assets_hint,
        name = escape(&level.name),
        namespace = namespace(level, options),
        entries = entries,
        setup = setup,
    );

    let mut files=vec![
        UnityFile {
            path: "Editor/HeronBrushLevelImporter.cs".to_string(),
            contents: script,
        },
        UnityFile {
            path: format!("{}.json", level.name),
            contents: level.to_json(),
        },
        UnityFile {
            path: format!("{}.gameplay.json", level.name),
            contents: live_data(level, collision),
        },
        UnityFile {path:"HeronBrushAnimationTrigger.cs".into(),contents:include_str!("unity_animation_trigger.cs").into()},
        UnityFile { path:"HeronBrushAnimation.cs".into(), contents:include_str!("unity_animation.cs").into() },
        UnityFile {path:"HeronBrushWater.cs".into(),contents:include_str!("unity_water.cs").into()},
        UnityFile {path:"HeronBrushWaterView.cs".into(),contents:include_str!("unity_water_view.cs").into()},
        UnityFile {path:"HeronBrushWater.shader".into(),contents:include_str!("unity_water.shader").into()},
        UnityFile { path:"HeronBrushMotionPlayer.cs".into(), contents:include_str!("unity_player.cs").into() },
        UnityFile { path:"HeronBrushMotionPlayer.md".into(), contents:"# Optional Unity platform walker\n\nCreate one HeronBrushMotionPlayer(importedRoot, controller) per level, with a CharacterController whose transform is at its feet, height 1.8, radius 0.35 and centre (0, 0.9, 0). Put the player on layer 31 so support raycasts exclude it. Call Step(dt, worldWalkDirection, jump) once from your fixed tick. The helper owns the level animation/trigger clock and disables their automatic ticks until Dispose. It carries grounded players by the full support pose, including rotation; jumping detaches. Ground contact uses a downward bias to clear the CharacterController skin gap. Place(feet) resets position and velocity. Games with several players should own their shared animation clock and carrying system.\n\nThe helper tests native exported trigger hulls at the player's feet. The default trigger component still supports Unity entry callbacks and explicit Activate(). Advance(dt, feet) is for a game-owned manual clock with ManualUpdate enabled.\n".into()},
        UnityFile { path:"HeronBrushAnimation.md".into(),contents:"# Unity keyframe animations\n\nImport the whole export under Assets, then use Heron Brush > Build Level. The importer adds HeronBrushAnimation and a kinematic Rigidbody to animated roots. Do not move HeronBrushAnimation.cs into an Editor folder: it is a runtime component.\n\nUse Play(\"open\"), Pause(), Resume(), Seek(seconds), Stop() from gameplay scripts. Autoplay respects start_on; once holds the final pose; Stop restores it. FixedUpdate moves the Rigidbody and its child colliders with the animation. Pivot coordinates follow the collection parent frame, so move the whole level using its common parent. The importer wires func_button, trigger_once and trigger_multiple targets with HeronBrushAnimationTrigger. A CharacterController or a collider tagged Player activates them on contact. Call Activate() for explicit interaction. The generated component respects animation_action, animation_clip, animation_once and wait. Configure these in the Entity inspector: toggle, play a named clip, open, close, pause, resume or stop. wait is the repeat cooldown. Without these properties, named open/close clips toggle and other animations restart. Re-export and rebuilding the level replaces generated objects.\n".into()},
        UnityFile {
            path: "HeronBrushEntity.cs".into(),
            contents: "// Generated metadata for common entities. Attach gameplay scripts to consume it.\nusing UnityEngine;\npublic sealed class HeronBrushEntity : MonoBehaviour { [TextArea] public string entityJson; }\n".into(),
        },
        UnityFile {path:"HeronBrushLevelRoot.cs".into(),contents:"using UnityEngine;\npublic sealed class HeronBrushLevelRoot : MonoBehaviour { public string sourceGuid; }\n".into()},
    ];
    if materials
        .values()
        .any(|material| material.pbr.as_ref().is_some_and(|pbr| pbr.double_sided))
    {
        files.push(UnityFile {
            path: "HeronBrushPbr.shader".into(),
            contents: include_str!("unity_pbr.shader").into(),
        });
    }
    let namespace = namespace(level, options);
    for file in &mut files {
        if file.path.ends_with(".cs") {
            file.contents = if file.contents.contains("#if UNITY_EDITOR") {
                file.contents
                    .replace(
                        "#if UNITY_EDITOR",
                        &format!("#if UNITY_EDITOR\nnamespace {namespace} {{"),
                    )
                    .replace("#endif", "}\n#endif")
            } else {
                format!("namespace {namespace} {{\n{}\n}}\n", file.contents)
            };
        }
    }
    files
}

fn property_literal(value: &crate::level::Value) -> String {
    use crate::level::Value;
    match value {
        Value::Bool(value) => value.to_string(),
        Value::Int(value) => format!("{value}L"),
        Value::Float(value) => format!("{value:.17}d"),
        Value::String(value) => format!("\"{}\"", escape(value)),
        Value::Array(values) => format!(
            "new object[]{{{}}}",
            values
                .iter()
                .map(property_literal)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Vec2(values) => format!(
            "new object[]{{{}}}",
            values
                .iter()
                .map(|value| format!("{value:.17}d"))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Vec3(values) => format!(
            "new object[]{{{}}}",
            values
                .iter()
                .map(|value| format!("{value:.17}d"))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Color(values) => format!(
            "new object[]{{{}}}",
            values
                .iter()
                .map(|value| format!("{value:.17}d"))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

/// Stable per-export namespace prevents runtime class collisions across levels.
pub fn namespace(level: &Level, options: &UnityOptions) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in format!("{}\n{}", options.assets_hint.replace('\\', "/"), level.name).bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("HeronBrush.Level_{hash:016x}")
}

/// Source stores [pitch X, yaw Y, roll Z], with matrix Ry * Rx * Rz.
/// Unity Quaternion.Euler applies Z, X, Y in that order (the same product).
/// Conjugating by the OBJ importer's X reflection preserves pitch and negates
/// yaw and roll. Sharing this conversion keeps file and live exports identical.
/// https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Quaternion.Euler.html
fn unity_euler_degrees([pitch, yaw, roll]: [f64; 3]) -> [f64; 3] {
    [pitch.to_degrees(), -yaw.to_degrees(), -roll.to_degrees()]
}

/// Escapes a string for a C# string literal.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Data-only snapshots for the Unity live addon: no C# regeneration/domain reload.
pub fn live_data(level: &Level, collision: &[CollisionMesh]) -> String {
    fn property(name: &str, value: &crate::level::Value) -> serde_json::Value {
        use crate::level::Value;
        let typed = serde_json::to_value(value).expect("entity property JSON");
        let raw = &typed["value"];
        let items = match value {
            Value::Array(values) => values
                .iter()
                .map(|value| property("", value))
                .collect::<Vec<_>>(),
            Value::Vec2(values) => values
                .iter()
                .map(|value| property("", &Value::Float(*value)))
                .collect(),
            Value::Vec3(values) => values
                .iter()
                .map(|value| property("", &Value::Float(*value)))
                .collect(),
            Value::Color(values) => values
                .iter()
                .map(|value| property("", &Value::Float(*value)))
                .collect(),
            _ => vec![],
        };
        serde_json::json!({"name":name,"kind":typed["type"],"value":raw.as_str().map(str::to_owned).unwrap_or_else(||raw.to_string()),"items":items})
    }
    let entities: Vec<_> = level.entities.iter().enumerate().map(|(index, e)| {
        let t = &e.transform;
        let light = super::common_entities::light(e).map(|l| serde_json::json!({
            "kind":l.kind,"color":l.color,"energy":l.energy,"range":l.range,
            "cone":l.half_angle*2.,"shadows":l.shadows
        }));
        let mut shapes = vec![];
        if let Some(c) = collision.get(index) {
            if !c.triangles.is_empty() { shapes.push(serde_json::json!({"convex":false,"points":c.triangles.iter().flatten().copied().collect::<Vec<_>>()})); }
            for hull in &c.hulls { if hull.len() >= 4 { shapes.push(serde_json::json!({"convex":true,"points":hull.iter().flatten().copied().collect::<Vec<_>>()})); } }
        }
        let properties: Vec<_> = e.properties.iter().map(|(name,value)|property(name,value)).collect();
        let text=|key|e.properties.get(key).and_then(|value|match value{crate::level::Value::String(value)=>Some(value.as_str()),_=>None});
        let mass=e.properties.get("unity_mass").and_then(|value|match value{crate::level::Value::Float(value)=>Some(*value),crate::level::Value::Int(value)=>Some(*value as f64),_=>None});
        serde_json::json!({"name":e.name,"classname":e.classname,"properties":properties,"targets":e.targets,"mesh":e.mesh,"metadata":serde_json::to_string(e).expect("entity JSON"),
            "prefab":prefab_path(e),"bodyType":text("unity_body_type"),"mass":mass.unwrap_or(0.),
            "position":[-t.translation[0],t.translation[1],t.translation[2]],
            "rotation":unity_euler_degrees(t.rotation_yxz),
            "scale":t.scale,"light":light,"shapes":shapes,"sensor":e.collision.as_ref().is_some_and(|c|c.sensor)})
    }).collect();
    serde_json::json!({"version":1,"name":level.name,"scale":level.scale,"entities":entities})
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{Entity, Transform};
    #[test]
    fn native_prefab_scene_mapping_is_the_same_in_file_and_live() {
        let mut level = a_level();
        level.entities[0].scene = Some("res://Assets/Puzzle.prefab".into());
        validate(&level).unwrap();
        let script = &write(&level, &[], &UnityOptions::default())[0].contents;
        assert!(script.contains(", false, false, \"Assets/Puzzle.prefab\")"));
        let live: serde_json::Value = serde_json::from_str(&live_data(&level, &[])).unwrap();
        assert_eq!(live["entities"][0]["prefab"], "Assets/Puzzle.prefab");
    }

    fn a_level() -> Level {
        Level {
            format: 1,
            generator: "test".into(),
            name: "arena".into(),
            scale: 32.0,
            textures: Vec::new(),
            entities: vec![Entity {
                name: "worldspawn".into(),
                classname: "worldspawn".into(),
                group: Vec::new(),
                transform: Transform {
                    translation: [1.0, 2.0, 3.0],
                    ..Transform::default()
                },
                solid: true,
                mesh: Some("arena/worldspawn.obj".into()),
                surfaces: Vec::new(),
                collision: None,
                properties: Default::default(),
                targets: Vec::new(),
                scene: None,
                tags: Vec::new(),
            }],
        }
    }

    #[test]
    fn the_script_bakes_the_placement_with_x_negated() {
        let files = write(&a_level(), &[], &UnityOptions::default());
        let script = &files[0].contents;
        assert!(files[0].path.ends_with("HeronBrushLevelImporter.cs"));
        assert!(script.contains("new Vector3(-1.000000f, 2.000000f, 3.000000f)"));
        assert!(script.contains("arena/worldspawn.obj"));
        assert!(script.contains("MenuItem(\"Heron Brush/Build Level: arena\")"));
    }

    #[test]
    fn file_and_live_transforms_match_godot_for_arbitrary_points() {
        use glam::{DMat3, DQuat, DVec3};
        let mut level = a_level();
        let t = &mut level.entities[0].transform;
        t.rotation_yxz = [0.37, -1.12, 0.64];
        t.scale = [2.0, 0.7, 3.1];
        let t = t.clone();
        let script = &write(&level, &[], &UnityOptions::default())[0].contents;
        let entry = script.lines().find(|l| l.contains("new Entry(")).unwrap();
        let vectors: Vec<DVec3> = entry
            .split("new Vector3(")
            .skip(1)
            .map(|part| {
                let v: Vec<f64> = part
                    .split(')')
                    .next()
                    .unwrap()
                    .split(',')
                    .map(|n| n.trim().trim_end_matches('f').parse().unwrap())
                    .collect();
                DVec3::new(v[0], v[1], v[2])
            })
            .collect();
        assert_eq!(vectors.len(), 3);
        let live: serde_json::Value = serde_json::from_str(&live_data(&level, &[])).unwrap();
        for (i, key) in ["position", "rotation", "scale"].iter().enumerate() {
            for axis in 0..3 {
                assert!(
                    (live["entities"][0][*key][axis].as_f64().unwrap() - vectors[i][axis]).abs()
                        < 1e-6
                );
            }
        }
        // Independent reference: the Godot writer's actual row-major basis.
        let [x, y, z] = t.rotation_yxz;
        let basis = crate::scene::basis_from_euler_yxz(crate::math::Vec3::new(x, y, z));
        let source = DMat3::from_cols_array(&basis).transpose();
        // Unity documentation specifies extrinsic Z, then X, then Y.
        let e = vectors[1] * (std::f64::consts::PI / 180.0);
        let rotation =
            DQuat::from_rotation_y(e.y) * DQuat::from_rotation_x(e.x) * DQuat::from_rotation_z(e.z);
        let mirror = |p: DVec3| DVec3::new(-p.x, p.y, p.z);
        for point in [DVec3::X, DVec3::Y, DVec3::Z, DVec3::new(0.3, -2., 4.)] {
            let expected = mirror(
                source * (DVec3::from_array(t.scale) * point) + DVec3::from_array(t.translation),
            );
            let actual = rotation * (vectors[2] * mirror(point)) + vectors[0];
            assert!(
                (actual - expected).length() < 1e-6,
                "{actual} != {expected}"
            );
        }
        assert!(script.contains("node.transform.localScale = entry.scale;"));
        // A Unity light emits along +Z; its local half-turn must produce the
        // transformed source -Z direction even when pitch and roll are nonzero.
        let light = rotation * DQuat::from_rotation_y(std::f64::consts::PI);
        assert!((light * DVec3::Z - mirror(source * -DVec3::Z)).length() < 1e-6);
    }

    #[test]
    fn generated_materials_get_normal_maps_clamped_paint_and_a_lasting_glow() {
        let generated = |repeat| crate::materials::GeneratedMaterial {
            path: String::new(),
            width: 1,
            height: 1,
            png: Vec::new(),
            alpha: false,
            surface: None,
            repeat,
            pbr: None,
        };
        let mut materials = indexmap::IndexMap::new();
        materials.insert("bbpaint_texture".to_string(), generated(true));
        materials.insert("bbpaint_paint".to_string(), generated(false));
        let options = UnityOptions {
            normal_maps: vec!["bbpaint_texture".into()],
            ..UnityOptions::default()
        };
        let script = &write_with_materials(&a_level(), &[], &options, &materials)[0].contents;
        assert!(script.contains("ApplyNormal(root, here, \"bbpaint_texture\", \"arena_paint\");"));
        assert!(script.contains("private static void ApplyNormal("));
        assert!(script.contains("TextureImporterType.NormalMap"));
        // Only the baked paint clamps; a texture keeps tiling.
        assert!(script.contains("ClampPaint(here, \"arena_paint\", new[] { \"bbpaint_paint\" });"));
        // Unity's Standard editor drops _EMISSION from a material flagged
        // EmissiveIsBlack, the flag an OBJ material starts with.
        assert!(script.contains("replacement.globalIlluminationFlags = emission > 0f"));
    }

    #[test]
    fn the_manifest_rides_along_as_json() {
        let files = write(&a_level(), &[], &UnityOptions::default());
        assert_eq!(files[1].path, "arena.json");
        assert!(files[1].contents.contains("\"worldspawn\""));
    }
}
