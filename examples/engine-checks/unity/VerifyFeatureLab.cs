// Checks the feature lab's Unity export in the engine, station by station.
// prepare_feature_lab.py installs it; run VerifyFeatureLab.Run with -nographics,
// then VerifyFeatureLab.Render with a GPU for the captures. See the README.
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using Object = UnityEngine.Object;

public static class VerifyFeatureLab
{
    const string Scene = "Assets/FeatureLab.unity";
    static readonly List<string> Lines = new List<string>();
    static readonly Dictionary<string, bool> Verdicts = new Dictionary<string, bool>();
    static readonly Dictionary<string, Color32[]> Images = new Dictionary<string, Color32[]>();
    static readonly Dictionary<string, Vector2Int> Sizes = new Dictionary<string, Vector2Int>();
    static Dictionary<string, object> reference;

    static string Project => Directory.GetParent(Application.dataPath).FullName;

    // The level's frame is the map's turned to Y up and mirrored on X, in metres.
    static Vector3 FromMap(double x, double y, double z) => new Vector3((float)(-y / 32), (float)(z / 32), (float)(x / 32));
    static Vector3 FromMap(List<object> v, int i) => FromMap(D(v[i]), D(v[i + 1]), D(v[i + 2]));
    static double D(object v) => Convert.ToDouble(v, CultureInfo.InvariantCulture);

    static void Report(string feature, bool pass, string evidence)
    {
        Verdicts[feature] = Verdicts.TryGetValue(feature, out var earlier) ? earlier && pass : pass;
        var line = $"HERON_FEATURE {feature} {(pass ? "PASS" : "FAIL")}: {evidence}";
        Lines.Add(line);
        if (pass) Debug.Log(line); else Debug.LogWarning(line);
    }

    static void Guard(string feature, Action check)
    {
        try { check(); }
        catch (Exception e) { Report(feature, false, "exception " + e.GetType().Name + ": " + e.Message); Debug.LogException(e); }
    }

    // ------------------------------------------------------------------ level access

    static string ExportFolder()
    {
        var space = typeof(HeronBrushLevelImporter).Namespace;
        foreach (var guid in AssetDatabase.FindAssets("HeronBrushLevelImporter t:MonoScript"))
        {
            var path = AssetDatabase.GUIDToAssetPath(guid);
            if (File.ReadAllText(path).Contains("namespace " + space))
                return Path.GetDirectoryName(Path.GetDirectoryName(path)).Replace('\\', '/');
        }
        throw new Exception("no importer script for " + space);
    }

    static GameObject Root() => Object.FindObjectsByType<HeronBrushLevelRoot>(FindObjectsSortMode.None).Single().gameObject;

    static readonly Regex TargetName = new Regex("\"targetname\":\\{\"type\":\"string\",\"value\":\"([^\"]*)\"");

    static Dictionary<string, GameObject> Stations(GameObject root)
    {
        var stations = new Dictionary<string, GameObject>();
        foreach (Transform child in root.transform)
        {
            var entity = child.GetComponent<HeronBrushEntity>();
            var match = entity == null ? null : TargetName.Match(entity.entityJson);
            if (match != null && match.Success) stations[match.Groups[1].Value] = child.gameObject;
        }
        return stations;
    }

    static Dictionary<string, object> EntityData(GameObject node) =>
        (Dictionary<string, object>)Json.Parse(node.GetComponent<HeronBrushEntity>().entityJson);

    // Pixels as Unity imports them: GetPixel order, bottom row first.
    static Color32[] Pixels(string path, out Vector2Int size)
    {
        if (!Images.TryGetValue(path, out var pixels))
        {
            var image = new Texture2D(2, 2, TextureFormat.RGBA32, false);
            ImageConversion.LoadImage(image, File.ReadAllBytes(path));
            pixels = image.GetPixels32();
            Images[path] = pixels;
            Sizes[path] = new Vector2Int(image.width, image.height);
            Object.DestroyImmediate(image);
        }
        size = Sizes[path];
        return pixels;
    }

    struct Surface { public Renderer renderer; public Material material; public Vector3[] corners; public Vector2[] uvs; public Vector3 normal; }

    // Every drawn triangle of a node in world space, with its material and UVs.
    static List<Surface> Triangles(GameObject node)
    {
        var all = new List<Surface>();
        foreach (var filter in node.GetComponentsInChildren<MeshFilter>())
        {
            var renderer = filter.GetComponent<MeshRenderer>();
            if (renderer == null || !renderer.enabled) continue;
            var mesh = filter.sharedMesh;
            var vertices = mesh.vertices; var uv = mesh.uv; var normals = mesh.normals;
            for (int sub = 0; sub < mesh.subMeshCount; sub++)
            {
                var indices = mesh.GetTriangles(sub);
                for (int i = 0; i < indices.Length; i += 3)
                {
                    var corners = new Vector3[3]; var uvs = new Vector2[3];
                    for (int k = 0; k < 3; k++) { corners[k] = filter.transform.TransformPoint(vertices[indices[i + k]]); uvs[k] = uv.Length > 0 ? uv[indices[i + k]] : Vector2.zero; }
                    var normal = filter.transform.TransformDirection(normals[indices[i]]).normalized;
                    all.Add(new Surface { renderer = renderer, material = renderer.sharedMaterials[sub], corners = corners, uvs = uvs, normal = normal });
                }
            }
        }
        return all;
    }

    static List<Vector3[]> ColliderTriangles(MeshCollider collider)
    {
        var mesh = collider.sharedMesh; var vertices = mesh.vertices; var indices = mesh.triangles;
        var result = new List<Vector3[]>();
        for (int i = 0; i < indices.Length; i += 3)
            result.Add(new[] { 0, 1, 2 }.Select(k => collider.transform.TransformPoint(vertices[indices[i + k]])).ToArray());
        return result;
    }

    // Which sides of the triangles' bounding box they cover.
    static string Sides(IEnumerable<Vector3[]> triangles)
    {
        var list = triangles.ToList();
        var points = list.SelectMany(t => t).ToList();
        var lo = new Vector3(points.Min(p => p.x), points.Min(p => p.y), points.Min(p => p.z));
        var hi = new Vector3(points.Max(p => p.x), points.Max(p => p.y), points.Max(p => p.z));
        var sides = new SortedSet<string>();
        foreach (var t in list)
            for (int axis = 0; axis < 3; axis++)
            {
                if (t.All(p => Mathf.Abs(p[axis] - lo[axis]) < 1e-4f)) sides.Add("-" + "xyz"[axis]);
                if (t.All(p => Mathf.Abs(p[axis] - hi[axis]) < 1e-4f)) sides.Add("+" + "xyz"[axis]);
            }
        return string.Join(" ", sides);
    }

    static bool Barycentric(Vector3 p, Vector3[] t, out Vector3 weights)
    {
        Vector3 v0 = t[1] - t[0], v1 = t[2] - t[0], v2 = p - t[0];
        float d00 = Vector3.Dot(v0, v0), d01 = Vector3.Dot(v0, v1), d11 = Vector3.Dot(v1, v1);
        float d20 = Vector3.Dot(v2, v0), d21 = Vector3.Dot(v2, v1), den = d00 * d11 - d01 * d01;
        float v = (d11 * d20 - d01 * d21) / den, w = (d00 * d21 - d01 * d20) / den;
        weights = new Vector3(1 - v - w, v, w);
        var normal = Vector3.Cross(v0, v1).normalized;
        return Mathf.Abs(Vector3.Dot(v2, normal)) < 1e-3f && weights.x > -1e-4f && weights.y > -1e-4f && weights.z > -1e-4f;
    }

    // The colour Unity's material gives a point: albedo texel under the
    // interpolated UV (point sampled, with the texture's own wrap) times _Color.
    static bool SampleAt(List<Surface> surfaces, Vector3 point, Vector3 normal, bool filtered, out Color32 colour, out Material material)
    {
        colour = default; material = null;
        foreach (var s in surfaces)
        {
            if (Vector3.Dot(s.normal, normal) < 0.99f || !Barycentric(point, s.corners, out var w)) continue;
            var uv = s.uvs[0] * w.x + s.uvs[1] * w.y + s.uvs[2] * w.z;
            var path = AssetDatabase.GetAssetPath(s.material.mainTexture);
            var importer = (TextureImporter)AssetImporter.GetAtPath(path);
            var pixels = Pixels(path, out var size);
            Func<float, int, int> texel = (c, n) => importer.wrapMode == TextureWrapMode.Repeat
                ? (int)Mathf.Repeat(Mathf.Floor(c * n), n) : Mathf.Clamp((int)Mathf.Floor(c * n), 0, n - 1);
            Color p = pixels[texel(uv.y, size.y) * size.x + texel(uv.x, size.x)];
            if (filtered)
            {
                // Bilinear, between the four texel centres around the point.
                float fx = uv.x * size.x - .5f, fy = uv.y * size.y - .5f;
                float ax = fx - Mathf.Floor(fx), ay = fy - Mathf.Floor(fy);
                Color At(float x, float y) => pixels[texel((y + .5f) / size.y, size.y) * size.x + texel((x + .5f) / size.x, size.x)];
                float x0 = Mathf.Floor(fx), y0 = Mathf.Floor(fy);
                p = Color.Lerp(Color.Lerp(At(x0, y0), At(x0 + 1, y0), ax), Color.Lerp(At(x0, y0 + 1), At(x0 + 1, y0 + 1), ax), ay);
            }
            var tint = s.material.HasProperty("_Color") ? s.material.color : Color.white;
            colour = new Color32((byte)Mathf.RoundToInt(p.r * tint.r * 255), (byte)Mathf.RoundToInt(p.g * tint.g * 255), (byte)Mathf.RoundToInt(p.b * tint.b * 255), 255);
            material = s.material;
            return true;
        }
        return false;
    }

    // Compares the editor's colour at points inside each face of a station.
    static void MatchEditorColours(GameObject node, string station, Func<int, Vector3, int> tolerance, string feature, string what, bool filtered = false)
    {
        var surfaces = Triangles(node);
        int points = 0, faces = 0, worst = 0; var problems = new List<string>();
        foreach (Dictionary<string, object> face in (List<object>)reference["faces"])
        {
            if ((string)face["station"] != station) continue;
            faces++;
            var p = (List<object>)face["points"]; var c = (List<object>)face["colours"]; var n = (List<object>)face["normal"];
            var normal = FromMap(D(n[0]), D(n[1]), D(n[2])).normalized;
            for (int i = 0; i < p.Count; i += 3)
            {
                var point = FromMap(p, i);
                if (!SampleAt(surfaces, point, normal, filtered, out var got, out _)) { problems.Add($"{face["texture"]} {normal}: no drawn surface at {point}"); continue; }
                int error = Math.Max(Math.Abs(got.r - (int)D(c[i])), Math.Max(Math.Abs(got.g - (int)D(c[i + 1])), Math.Abs(got.b - (int)D(c[i + 2]))));
                worst = Math.Max(worst, error); points++;
                if (error > tolerance(faces, normal)) problems.Add($"{face["texture"]} {normal} at {point}: {got} want {D(c[i])},{D(c[i + 1])},{D(c[i + 2])}");
            }
        }
        Report(feature, problems.Count == 0 && points > 0,
            problems.Count == 0 ? $"{what}: {points} points on {faces} faces match the editor's colour, largest difference {worst}/255"
                : $"{problems.Count} of {points + problems.Count} points differ: " + string.Join("; ", problems.Take(4)));
    }

    // ------------------------------------------------------------------ edit-mode checks

    public static void Run()
    {
        try
        {
            reference = (Dictionary<string, object>)Json.Parse(File.ReadAllText(Environment.GetEnvironmentVariable("HERON_FEATURE_REFERENCE") ?? Path.Combine(Project, "feature_lab_reference.json")));
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var export = ExportFolder();
            var messages = new List<string>();
            Application.LogCallback listen = (message, stack, type) => { if (type != LogType.Log) messages.Add(type + ": " + message); };
            Application.logMessageReceived += listen;
            try { HeronBrushLevelImporter.Build(); } finally { Application.logMessageReceived -= listen; }
            var root = Root();
            var stations = Stations(root);
            var level = (Dictionary<string, object>)Json.Parse(File.ReadAllText(Path.Combine(export, "feature_lab.json")));
            var entities = (List<object>)level["entities"];
            Report("importer", root.transform.childCount == entities.Count && messages.Count == 0,
                $"Build Level made '{root.name}' with {root.transform.childCount} of {entities.Count} entities; warnings and errors: {(messages.Count == 0 ? "none" : string.Join(" | ", messages))}");
            Guard("obj import", () => CheckImport(root, entities, export));
            Guard("textures", () => CheckTextures(stations["fl_textures"], export));
            Guard("colours", () => CheckColours(stations["fl_colours"]));
            // A gradient is continuous in the editor and a 64-pixel bake here, drawn filtered.
            Guard("gradients", () => MatchEditorColours(stations["fl_gradients"], "fl_gradients", (f, n) => 6, "gradients", "baked gradients, sampled bilinear", true));
            Guard("surface", () => CheckSurface(stations["fl_surface"], level, export));
            Guard("normal", () => CheckNormal(stations["fl_normal"], export));
            Physics.SyncTransforms();
            Guard("solid", () => CheckSolid(stations["fl_solid"], root));
            Guard("clip", () => CheckClip(stations["fl_clip"]));
            Guard("skip", () => CheckSkip(stations["fl_skip"]));
            Guard("hidden", () => CheckHidden(stations["fl_hidden"]));
            Guard("trigger", () => CheckTrigger(stations));
            Guard("animation", () => CheckPoses(stations));
            Guard("lights", () => CheckLights(stations));
            Guard("water", () => CheckWater(stations["fl_water"]));
            Guard("properties", () => CheckProperties(stations));
            Guard("links", () => CheckLinks(stations));
            Guard("models", () => CheckModels(stations, export));
            EditorSceneManager.SaveScene(UnityEngine.SceneManagement.SceneManager.GetActiveScene(), Scene);
            // Play the saved scene: OnTriggerEnter and automatic playback need Play Mode.
            EditorSceneManager.OpenScene(Scene);
            EditorSettings.enterPlayModeOptionsEnabled = true;
            EditorSettings.enterPlayModeOptions = EnterPlayModeOptions.DisableDomainReload | EnterPlayModeOptions.DisableSceneReload;
            started = EditorApplication.timeSinceStartup;
            EditorApplication.update += PlayTick;
            EditorApplication.isPlaying = true;
        }
        catch (Exception e) { Debug.LogException(e); EditorApplication.Exit(2); }
    }

    static void CheckImport(GameObject root, List<object> entities, string export)
    {
        var problems = new List<string>(); int meshes = 0, slots = 0;
        foreach (Dictionary<string, object> entity in entities)
        {
            var node = root.transform.Find((string)entity["name"]);
            if (node == null) { problems.Add("missing node " + entity["name"]); continue; }
            var wanted = entity.TryGetValue("surfaces", out var s) && s != null
                ? new SortedSet<string>(((List<object>)s).Select(x => (string)((Dictionary<string, object>)x)["texture"])) : new SortedSet<string>();
            var renderers = node.GetComponentsInChildren<MeshRenderer>();
            var got = new SortedSet<string>(renderers.SelectMany(r => r.sharedMaterials).Select(m => m == null ? "(none)" : m.name));
            if (!wanted.SetEquals(got)) problems.Add($"{entity["name"]} materials {string.Join(",", got)} want {string.Join(",", wanted)}");
            if (renderers.Length > 0) meshes++;
            foreach (var material in renderers.SelectMany(r => r.sharedMaterials).Where(m => m != null))
            {
                slots++;
                var texture = AssetDatabase.GetAssetPath(material.mainTexture);
                if (texture != $"{export}/feature_lab_paint/{material.name}.png") problems.Add($"{material.name} albedo is '{texture}'");
                if (material.shader == null || material.shader.name == "Hidden/InternalErrorShader") problems.Add($"{material.name} has no working shader");
            }
        }
        Report("obj import", problems.Count == 0,
            problems.Count == 0 ? $"{meshes} OBJ meshes; all {slots} material slots named as the MTL says, with their feature_lab_paint PNG as albedo" : string.Join("; ", problems.Take(6)));
    }

    static readonly Dictionary<string, Vector2> TextureSpans = new Dictionary<string, Vector2> {
        {"wood", new Vector2(1, 1)}, {"metal", new Vector2(1, 1)}, {"crate", new Vector2(Mathf.Sqrt(2), Mathf.Sqrt(2))},
        {"wall", new Vector2(2, .5f)}, {"dirt", new Vector2(.5f, .5f)}, {"hazard", new Vector2(1, 1)} };

    static void CheckTextures(GameObject node, string export)
    {
        var problems = new List<string>(); var found = new List<string>();
        var sources = TextureSpans.Keys.Concat(new[] { "grass" }).ToDictionary(t => t, t => Pixels($"{export}/textures/{t}.png", out _));
        foreach (var group in Triangles(node).GroupBy(s => s.material))
        {
            var path = AssetDatabase.GetAssetPath(group.Key.mainTexture);
            var pixels = Pixels(path, out _);
            var texture = sources.FirstOrDefault(p => p.Value.Length == pixels.Length && p.Value.SequenceEqual(pixels)).Key;
            if (texture == null) { problems.Add($"{group.Key.name}: pixels match no source texture"); continue; }
            found.Add(texture);
            var importer = (TextureImporter)AssetImporter.GetAtPath(path);
            if (importer.wrapMode != TextureWrapMode.Repeat) problems.Add($"{texture} wraps {importer.wrapMode}");
            if (!TextureSpans.TryGetValue(texture, out var want)) continue;
            var uvs = group.SelectMany(s => s.uvs).ToList();
            var span = new Vector2(uvs.Max(u => u.x) - uvs.Min(u => u.x), uvs.Max(u => u.y) - uvs.Min(u => u.y));
            if (Mathf.Abs(span.x - want.x) > .02f || Mathf.Abs(span.y - want.y) > .02f) problems.Add($"{texture} UV span {span} want {want}");
        }
        if (!TextureSpans.Keys.All(found.Contains)) problems.Add("textures found: " + string.Join(",", found));
        Report("textures", problems.Count == 0, problems.Count == 0
            ? $"{found.Count} surfaces hold the source pixels and repeat; UV spans 1x1, 1.414x1.414, 2x0.5, 0.5x0.5 as the face scales say"
            : string.Join("; ", problems));
        // The texel under each point: offset, scale and rotation as the editor maps them.
        MatchEditorColours(node, "fl_textures", (f, n) => 2, "textures", "texel lookups");
    }

    static void CheckColours(GameObject node)
    {
        // The face looking down (map -Z) is the translucent one, blended within 4/255.
        MatchEditorColours(node, "fl_colours", (face, n) => n.y < -0.99f ? 4 : 2, "colours", "flat and translucent paint");
        var wraps = node.GetComponentsInChildren<MeshRenderer>().SelectMany(r => r.sharedMaterials)
            .Select(m => ((TextureImporter)AssetImporter.GetAtPath(AssetDatabase.GetAssetPath(m.mainTexture))).wrapMode).Distinct().ToList();
        Report("colours", wraps.All(w => w == TextureWrapMode.Clamp), "paint images wrap " + string.Join(",", wraps) + " (the contract says generated paint clamps)");
    }

    static void CheckSurface(GameObject node, Dictionary<string, object> level, string export)
    {
        var problems = new List<string>(); int count = 0;
        var materials = node.GetComponentsInChildren<MeshRenderer>().SelectMany(r => r.sharedMaterials).ToList();
        foreach (Dictionary<string, object> texture in (List<object>)level["textures"])
        {
            if (!texture.TryGetValue("surface", out var s) || s == null) continue;
            var f = (List<object>)s; float roughness = (float)D(f[0]), metallic = (float)D(f[1]), emission = (float)D(f[2]);
            var name = (string)texture["name"];
            var m = materials.FirstOrDefault(x => x.name == name);
            if (m == null) { problems.Add("no slot " + name); continue; }
            count++;
            if (AssetDatabase.GetAssetPath(m) != $"{export}/Materials/{name}.mat") problems.Add($"{name} is not the generated material");
            if (m.shader.name != "Standard") problems.Add($"{name} shader {m.shader.name}");
            if (Mathf.Abs(m.GetFloat("_Metallic") - metallic) > 1e-4f) problems.Add($"{name} metallic {m.GetFloat("_Metallic")}");
            if (Mathf.Abs(m.GetFloat("_Glossiness") - (1 - roughness)) > 1e-4f) problems.Add($"{name} smoothness {m.GetFloat("_Glossiness")}");
            bool glows = m.IsKeywordEnabled("_EMISSION");
            if (glows != emission > 0) problems.Add($"{name} emission keyword {glows}");
            if (emission > 0 && (Vector4.Distance(m.GetColor("_EmissionColor"), new Vector4(emission, emission, emission, emission)) > 1e-3f || m.GetTexture("_EmissionMap") != m.mainTexture))
                problems.Add($"{name} emission {m.GetColor("_EmissionColor")} map {m.GetTexture("_EmissionMap")}");
        }
        Report("surface", problems.Count == 0 && count == 4, problems.Count == 0
            ? $"{count} ApplySurface materials on Standard: metallic and smoothness = 1 - roughness for (0.2,0.9) (0.9,0) (0.65,0) (0.05,1); emission 3 on the albedo map, off elsewhere"
            : string.Join("; ", problems));
        MatchEditorColours(node, "fl_surface", (f, n) => 2, "surface", "albedo under the overrides");
    }

    static void CheckNormal(GameObject node, string export)
    {
        MatchEditorColours(node, "fl_normal", (f, n) => 2, "normal", "brick_wall albedo");
        var problems = new List<string>();
        var material = node.GetComponentInChildren<MeshRenderer>().sharedMaterial;
        var bump = material.HasProperty("_BumpMap") ? material.GetTexture("_BumpMap") as Texture2D : null;
        if (bump == null) { Report("normal", false, $"{material.name} ({material.shader.name}) has no _BumpMap"); return; }
        var path = AssetDatabase.GetAssetPath(bump);
        var importer = (TextureImporter)AssetImporter.GetAtPath(path);
        if (importer.textureType != TextureImporterType.NormalMap) problems.Add("import type " + importer.textureType);
        if (!material.IsKeywordEnabled("_NORMALMAP")) problems.Add("_NORMALMAP is off");
        if (bump.width != 512 || bump.height != 512) problems.Add($"size {bump.width}x{bump.height}");
        // Unity's normal maps are OpenGL (+Y): the file must be the GL source unchanged.
        var source = Pixels($"{export}/textures/brick_wall_normal.png", out _);
        var got = Pixels(path, out _);
        if (!source.Select(p => (p.r, p.g, p.b)).SequenceEqual(got.Select(p => (p.r, p.g, p.b)))) problems.Add("pixels differ from brick_wall_normal.png");
        Report("normal", problems.Count == 0, problems.Count == 0
            ? $"_BumpMap {Path.GetFileName(path)} imported as NormalMap, 512x512, _NORMALMAP on, pixels equal to the OpenGL source"
            : string.Join("; ", problems));
    }

    static MeshCollider[] Colliders(GameObject node) => node.GetComponentsInChildren<MeshCollider>();

    static string Describe(MeshCollider c) => $"{(c.convex ? "convex" : "triangle")}{(c.isTrigger ? " trigger" : "")} {c.bounds.size.x:0.###}x{c.bounds.size.y:0.###}x{c.bounds.size.z:0.###}";

    static RaycastHit? Down(Vector3 at)
    {
        return Physics.Raycast(at + Vector3.up * 10, Vector3.down, out var hit, 20, ~0, QueryTriggerInteraction.Ignore) ? hit : (RaycastHit?)null;
    }

    static void CheckSolid(GameObject node, GameObject root)
    {
        var c = Colliders(node);
        bool ok = c.Length == 1 && c[0].convex && !c[0].isTrigger && Vector3.Distance(c[0].bounds.size, new Vector3(2, 2, 2)) < 1e-3f;
        var hit = Down(node.transform.position);
        ok &= hit.HasValue && hit.Value.collider == c[0] && Mathf.Abs(hit.Value.point.y - (node.transform.position.y + 1)) < 1e-3f;
        var floor = root.transform.Find("worldspawn").GetComponent<MeshCollider>();
        var floorHit = Down(new Vector3(0, 0, 10));
        ok &= floor != null && !floor.convex && floorHit.HasValue && floorHit.Value.collider == floor && Mathf.Abs(floorHit.Value.point.y) < 1e-3f;
        Report("solid", ok, $"fl_solid {string.Join(",", c.Select(Describe))}, ray down stops on it at y={hit?.point.y:0.###}; worldspawn {(floor ? Describe(floor) : "none")}, ray stops at y={floorHit?.point.y:0.###}");
    }

    static void CheckClip(GameObject node)
    {
        var c = Colliders(node);
        int renderers = node.GetComponentsInChildren<Renderer>().Length;
        var hit = Down(node.transform.position);
        bool ok = renderers == 0 && c.Length == 1 && c[0].convex && !c[0].isTrigger && Vector3.Distance(c[0].bounds.size, new Vector3(2, 3, 2)) < 1e-3f
            && hit.HasValue && hit.Value.collider == c[0];
        Report("clip", ok, $"fl_clip {renderers} renderers, {string.Join(",", c.Select(Describe))}, ray down stops on it at y={hit?.point.y:0.###}");
    }

    static void CheckSkip(GameObject node)
    {
        var render = Sides(Triangles(node).Select(s => s.corners));
        var c = Colliders(node);
        var collision = c.Length == 1 ? Sides(ColliderTriangles(c[0])) : "?";
        var hit = Down(node.transform.position);
        bool ok = render == "+x +z -x -y -z" && c.Length == 1 && !c[0].convex && !c[0].isTrigger && collision == render
            && !(hit.HasValue && hit.Value.collider == c[0] && hit.Value.point.y > node.transform.position.y);
        Report("skip", ok, $"fl_skip drawn sides [{render}], {string.Join(",", c.Select(Describe))} covering [{collision}]; a ray down through the skipped top {(hit.HasValue ? "stops on " + hit.Value.collider.name + " at y=" + hit.Value.point.y.ToString("0.###") : "hits nothing")}");
    }

    static void CheckHidden(GameObject node)
    {
        var render = Sides(Triangles(node).Select(s => s.corners));
        var c = Colliders(node);
        var collision = c.Length == 1 ? Sides(ColliderTriangles(c[0])) : "?";
        bool ok = render.Split(' ').Length == 5 && collision == "+x +y +z -x -y -z";
        Report("hidden", ok, $"fl_hidden drawn sides [{render}], triangle collision covers [{collision}]");
    }

    static void CheckTrigger(Dictionary<string, GameObject> stations)
    {
        var node = stations["fl_trigger"];
        var c = Colliders(node);
        var trigger = node.GetComponent<HeronBrushAnimationTrigger>();
        var door = stations["fl_door"].GetComponent<HeronBrushAnimation>();
        bool ok = node.GetComponentsInChildren<Renderer>().Length == 0 && c.Length == 1 && c[0].isTrigger && c[0].convex
            && trigger != null && trigger.targets.Length == 1 && trigger.targets[0] == door;
        Report("trigger", ok, $"fl_trigger draws nothing, {string.Join(",", c.Select(Describe))}, HeronBrushAnimationTrigger aimed at {string.Join(",", trigger.targets.Select(t => t.name))}");
    }

    static void CheckPoses(Dictionary<string, GameObject> stations)
    {
        int count = 0; double worst = 0; var problems = new List<string>();
        foreach (Dictionary<string, object> pose in (List<object>)reference["cases"])
        {
            var source = stations[(string)pose["entity"]];
            // A copy, so the scene's components start Play Mode untouched.
            var node = Object.Instantiate(source, source.transform.parent);
            try
            {
                var animation = node.GetComponent<HeronBrushAnimation>();
                var rest = node.transform.localToWorldMatrix;
                var clip = (string)pose["clip"];
                if (!animation.Play(clip == "" ? null : clip)) { problems.Add("no clip " + clip); continue; }
                animation.Seek(D(pose["time"])); animation.Advance(0);
                var restPoints = (List<object>)pose["rest"]; var posed = (List<object>)pose["posed"];
                for (int i = 0; i < 12; i += 3)
                {
                    var local = rest.inverse.MultiplyPoint3x4(FromMap(restPoints, i));
                    var error = Vector3.Distance(node.transform.localToWorldMatrix.MultiplyPoint3x4(local), FromMap(posed, i));
                    worst = Math.Max(worst, error);
                    if (error > 1e-3) problems.Add($"{pose["entity"]} {clip} t={pose["time"]} point {i / 3} off by {error:0.####} m");
                }
                count++;
            }
            finally { Object.DestroyImmediate(node); }
        }
        var mover = stations["fl_mover"].GetComponent<HeronBrushAnimation>();
        Report("animation", problems.Count == 0 && count > 0 && mover.dataJson.Contains("\"autoplay\":true"),
            problems.Count == 0 ? $"{count} poses (mover linear X, held Z, bezier turn over its 4 s loop; door open clip) match the editor's within {worst:0.######} m; mover autoplays"
                : string.Join("; ", problems.Take(5)));
    }

    static void CheckLights(Dictionary<string, GameObject> stations)
    {
        var point = stations["fl_light"].GetComponent<Light>();
        var spot = stations["fl_spot"].GetComponent<Light>();
        var problems = new List<string>();
        if (point == null || point.type != LightType.Point || Mathf.Abs(point.intensity - 2.5f) > 1e-4f || Mathf.Abs(point.range - 8) > 1e-4f
            || Vector4.Distance(point.color, new Color(1, 200 / 255f, 120 / 255f, 1)) > 1e-3f || Vector3.Distance(point.transform.position, FromMap(-160, -288, 160)) > 1e-4f)
            problems.Add($"point {point?.type} {point?.intensity} {point?.range} {point?.color} at {point?.transform.position}");
        // The editor aims angles "-60 45 0" (pitch down positive, yaw about +Z) along:
        double p = -60 * Math.PI / 180, y = 45 * Math.PI / 180;
        var aim = FromMap(Math.Cos(p) * Math.Cos(y), Math.Cos(p) * Math.Sin(y), -Math.Sin(p)).normalized;
        if (spot == null || spot.type != LightType.Spot || Mathf.Abs(spot.spotAngle - 60) > 1e-4f || Mathf.Abs(spot.range - 10) > 1e-4f
            || Mathf.Abs(spot.intensity - 3) > 1e-4f || Vector3.Distance(spot.transform.position, FromMap(-480, 64, 192)) > 1e-4f)
            problems.Add($"spot {spot?.type} {spot?.spotAngle} {spot?.range} {spot?.intensity} at {spot?.transform.position}");
        float off = spot == null ? 180 : Vector3.Angle(spot.transform.forward, aim);
        if (off > 0.01f) problems.Add($"spot shines along {spot?.transform.forward}, editor aims {aim}");
        Report("lights", problems.Count == 0, problems.Count == 0
            ? $"Point light 2.5, range 8, colour (255,200,120); Spot light 3, range 10, cone 60 degrees, shining along {aim} as the editor aims it ({off:0.####} degrees apart)"
            : string.Join("; ", problems));
    }

    static void CheckWater(GameObject node)
    {
        var water = node.GetComponent<HeronBrushWater>();
        var c = Colliders(node);
        var problems = new List<string>();
        if (water == null) { Report("water", false, "no HeronBrushWater"); return; }
        if (Mathf.Abs(water.Settings.swim_speed - 2.5f) > 1e-4f) problems.Add("swim_speed " + water.Settings.swim_speed);
        if (!water.Sample(node.transform.position, out float depth) || depth <= 0) problems.Add("centre is not under water");
        if (water.Sample(node.transform.position + Vector3.up * 3, out _)) problems.Add("above the volume is under water");
        if (c.Length != 1 || !c[0].isTrigger) problems.Add("collider " + string.Join(",", c.Select(Describe)));
        var faces = Triangles(node);
        if (faces.Count == 0 || faces.Any(f => f.normal.y < 0.99f)) problems.Add($"surface has {faces.Count} triangles, not all facing up");
        var shader = Shader.Find("HeronBrush/Water");
        if (shader == null || ShaderUtil.ShaderHasError(shader)) problems.Add("HeronBrush/Water shader missing or broken");
        Report("water", problems.Count == 0, problems.Count == 0
            ? $"HeronBrushWater swim_speed 2.5, centre {depth:0.##} m deep, sensor {Describe(c[0])}, {faces.Count} upward surface triangles, HeronBrush/Water compiles"
            : string.Join("; ", problems));
    }

    static object Typed(Dictionary<string, object> entity, string key, out string type)
    {
        var p = (Dictionary<string, object>)((Dictionary<string, object>)entity["properties"])[key];
        type = (string)p["type"];
        return p["value"];
    }

    static void CheckProperties(Dictionary<string, GameObject> stations)
    {
        var problems = new List<string>();
        void Want(string station, string key, string type, Func<object, bool> value)
        {
            var v = Typed(EntityData(stations[station]), key, out var t);
            if (t != type || !value(v)) problems.Add($"{station}.{key} is {t} {Json.Write(v)}");
        }
        Want("fl_probe", "charge", "int", v => v is long l && l == 7);
        Want("fl_probe", "armed", "bool", v => v is bool b && b);
        Want("fl_probe", "sequence", "array", v => Json.Write(v) == "[{\"type\":\"int\",\"value\":2},{\"type\":\"int\",\"value\":3},{\"type\":\"int\",\"value\":5}]" || Json.Write(v) == "[2,3,5]");
        Want("fl_probe", "label", "string", v => (string)v == "probe");
        Want("fl_probe", "note", "string", v => (string)v == "free text, with a comma");
        Want("fl_light", "light_energy", "float", v => v is double d && d == 2.5);
        Want("fl_light", "light_color", "color", v => ((List<object>)v).Select(D).Zip(new[] { 1, 200 / 255.0, 120 / 255.0, 1 }, (a, b) => Math.Abs(a - b)).Max() < 1e-9);
        Want("fl_camera", "fov", "float", v => v is double d && d == 62.5);
        Want("fl_camera", "current", "int", v => v is long l && l == 1);
        Want("fl_water", "swim_speed", "float", v => v is double d && d == 2.5);
        Want("fl_water", "water_priority", "int", v => v is long l && l == 3);
        Report("properties", problems.Count == 0, problems.Count == 0
            ? "HeronBrushEntity.entityJson keeps charge int 7, armed bool, sequence [2,3,5], both strings with the comma, light energy and camera fov as floats, camera current and water priority as ints"
            : string.Join("; ", problems));
    }

    static void CheckLinks(Dictionary<string, GameObject> stations)
    {
        var problems = new List<string>();
        void Link(string from, string to)
        {
            var targets = ((List<object>)EntityData(stations[from])["targets"]).Cast<string>().ToList();
            if (targets.Count != 1 || targets[0] != stations[to].name) problems.Add($"{from} targets {string.Join(",", targets)}, want {stations[to].name}");
        }
        Link("fl_wp_a", "fl_wp_b"); Link("fl_probe", "fl_wp_a"); Link("fl_trigger", "fl_door"); Link("fl_button", "fl_door");
        var door = stations["fl_door"].GetComponent<HeronBrushAnimation>();
        foreach (var name in new[] { "fl_trigger", "fl_button" })
        {
            var trigger = stations[name].GetComponent<HeronBrushAnimationTrigger>();
            if (trigger == null || trigger.targets.Length != 1 || trigger.targets[0] != door) problems.Add(name + " component is not wired to the door");
        }
        Report("links", problems.Count == 0, problems.Count == 0
            ? "fl_wp_a -> fl_wp_b, fl_probe -> fl_wp_a, fl_trigger and fl_button -> fl_door resolve to scene nodes; both trigger components hold the door's HeronBrushAnimation"
            : string.Join("; ", problems));
    }

    static void CheckModels(Dictionary<string, GameObject> stations, string export)
    {
        var problems = new List<string>();
        var a = stations["fl_model_a"]; var b = stations["fl_model_b"];
        var meshesA = a.GetComponentsInChildren<MeshFilter>().Select(f => f.sharedMesh).ToList();
        var meshesB = b.GetComponentsInChildren<MeshFilter>().Select(f => f.sharedMesh).ToList();
        if (meshesA.Count == 0 || !meshesA.SequenceEqual(meshesB)) problems.Add("copies do not share their mesh");
        if (Vector3.Distance(a.transform.localScale, Vector3.one * 32) > 1e-4f || Vector3.Distance(b.transform.localScale, Vector3.one * 48) > 1e-4f) problems.Add($"scales {a.transform.localScale} {b.transform.localScale}");
        if (Quaternion.Angle(a.transform.rotation, Quaternion.identity) > 1e-3f || Quaternion.Angle(b.transform.rotation, Quaternion.Euler(0, -45, 0)) > 1e-3f) problems.Add($"turns {a.transform.eulerAngles} {b.transform.eulerAngles}");
        Report("models", problems.Count == 0, problems.Count == 0
            ? $"both heron_model copies draw the same {meshesA.Count} mesh asset(s), scales 32 and 48, the second turned 45 degrees"
            : string.Join("; ", problems));
        var pbr = new List<string>(); int materials = 0;
        foreach (var m in a.GetComponentsInChildren<MeshRenderer>().SelectMany(r => r.sharedMaterials).Distinct())
        {
            materials++;
            var normal = m.GetTexture("_BumpMap");
            var normalType = normal == null ? (TextureImporterType?)null : ((TextureImporter)AssetImporter.GetAtPath(AssetDatabase.GetAssetPath(normal))).textureType;
            if (m.GetTexture("_MetallicGlossMap") == null || !m.IsKeywordEnabled("_METALLICGLOSSMAP")) pbr.Add(m.name + " metallic map");
            if (normal == null || normalType != TextureImporterType.NormalMap || !m.IsKeywordEnabled("_NORMALMAP")) pbr.Add(m.name + " normal map");
            if (m.GetTexture("_OcclusionMap") == null) pbr.Add(m.name + " occlusion map");
            if (m.GetTexture("_EmissionMap") == null || !m.IsKeywordEnabled("_EMISSION")) pbr.Add(m.name + " emission");
        }
        Report("model pbr", pbr.Count == 0 && materials > 0, pbr.Count == 0
            ? $"ApplyPbr gives both model materials metallic-smoothness, NormalMap-type normal, occlusion and emission maps on Standard"
            : "missing " + string.Join(", ", pbr));
    }

    // ------------------------------------------------------------------ Play Mode

    static double started, stageStart;
    static int stage;
    static CharacterController player;
    static Recorder recorder;
    static Dictionary<string, GameObject> live;
    static Vector3 moverRest;
    static Matrix4x4 doorRest;
    static Camera viewer;

    // Notes the triggers the player's capsule enters.
    public sealed class Recorder : MonoBehaviour
    {
        public readonly List<string> entered = new List<string>();
        void OnTriggerEnter(Collider other) { entered.Add(other.name); }
    }

    static void Place(Vector3 feet) { player.enabled = false; player.transform.position = feet; player.enabled = true; Physics.SyncTransforms(); }

    static void Next() { stage++; stageStart = EditorApplication.timeSinceStartup; }
    static double Elapsed => EditorApplication.timeSinceStartup - stageStart;

    static bool DoorAt(string clip, double time, out float worst)
    {
        worst = 0;
        foreach (Dictionary<string, object> pose in (List<object>)reference["cases"])
        {
            if ((string)pose["entity"] != "fl_door" || (string)pose["clip"] != clip || D(pose["time"]) != time) continue;
            var rest = (List<object>)pose["rest"]; var posed = (List<object>)pose["posed"];
            var door = live["fl_door"].transform;
            for (int i = 0; i < 12; i += 3)
                worst = Mathf.Max(worst, Vector3.Distance(door.localToWorldMatrix.MultiplyPoint3x4(doorRest.inverse.MultiplyPoint3x4(FromMap(rest, i))), FromMap(posed, i)));
            return worst < 2e-3f;
        }
        return false;
    }

    static void PlayTick()
    {
        try
        {
            if (EditorApplication.timeSinceStartup - started > 120) throw new Exception("Play Mode checks timed out at stage " + stage);
            if (!Application.isPlaying) return;
            switch (stage)
            {
                case 0:
                    live = Stations(Root());
                    moverRest = live["fl_mover"].transform.position;
                    doorRest = live["fl_door"].transform.localToWorldMatrix;
                    var body = new GameObject("Feature lab player");
                    player = body.AddComponent<CharacterController>();
                    player.height = 1.8f; player.radius = .35f; player.center = Vector3.up * .9f; player.skinWidth = .01f; player.minMoveDistance = 0;
                    recorder = body.AddComponent<Recorder>();
                    Place(new Vector3(-1.5f, .05f, -5.5f));
                    Next(); break;
                case 1: // walk into the trigger volume, which spans z -4 to -1
                    player.Move(new Vector3(0, -.01f, .05f));
                    if (player.transform.position.z > -3 && Elapsed > .5) Next();
                    break;
                case 2:
                {
                    var door = live["fl_door"].GetComponent<HeronBrushAnimation>();
                    var mover = live["fl_mover"].GetComponent<HeronBrushAnimation>();
                    bool entered = recorder.entered.Contains("trigger_multiple");
                    Report("trigger", entered && door.CurrentClip == "open",
                        $"in Play Mode the capsule's OnTriggerEnter saw [{string.Join(",", recorder.entered)}]; the door then plays '{door.CurrentClip}'");
                    float moved = Vector3.Distance(mover.transform.position, moverRest);
                    Report("animation", mover.IsPlaying && moved > .1f, $"in Play Mode the mover autoplays: playing={mover.IsPlaying}, clip time {mover.ClipTime:0.##} s, {moved:0.##} m from rest");
                    Next(); break;
                }
                case 3:
                    if (Elapsed < 1.2) break;
                {
                    var door = live["fl_door"].GetComponent<HeronBrushAnimation>();
                    bool open = DoorAt("open", 1, out float error);
                    Report("door", open && !door.IsPlaying, $"trigger: door ends the open clip at the editor's open pose (off by {error:0.#####} m)");
                    Place(new Vector3(-10.25f, .05f, -16.5f));
                    recorder.entered.Clear();
                    Next(); break;
                }
                case 4: // walk onto the button (z -18 to -17.5) from +Z, clear of the open door
                    player.Move(new Vector3(0, -.01f, -.04f));
                    if (player.transform.position.z < -17.4f && Elapsed > .5) Next();
                    break;
                case 5:
                    if (Elapsed < 1.2) break;
                {
                    var door = live["fl_door"].GetComponent<HeronBrushAnimation>();
                    bool shut = DoorAt("open", 0, out float error);
                    Report("door", recorder.entered.Contains("func_button") && door.CurrentClip == "close" && shut,
                        $"button: OnTriggerEnter saw [{string.Join(",", recorder.entered)}], the door toggled to '{door.CurrentClip}' and is back at rest (off by {error:0.#####} m)");
                    var water = live["fl_water"].GetComponent<HeronBrushWater>();
                    var shaders = water.GetComponentsInChildren<MeshRenderer>().SelectMany(r => r.sharedMaterials).Select(m => m.shader.name).Distinct().ToList();
                    Report("water", shaders.SequenceEqual(new[] { "HeronBrush/Water" }), "in Play Mode the surface draws with " + string.Join(",", shaders));
                    viewer = new GameObject("Water viewer").AddComponent<Camera>();
                    var view = viewer.gameObject.AddComponent<HeronBrushWaterView>();
                    view.Waters = new[] { water };
                    RenderSettings.fog = false;
                    viewer.transform.position = water.transform.position;
                    Next(); break;
                }
                case 6:
                    if (Elapsed < .3) break;
                {
                    var water = live["fl_water"].GetComponent<HeronBrushWater>();
                    bool under = RenderSettings.fog && RenderSettings.fogMode == FogMode.ExponentialSquared && Mathf.Abs(RenderSettings.fogDensity - water.Settings.fog_density) < 1e-4f
                        && Vector4.Distance(RenderSettings.fogColor, water.Tint) < 1e-4f;
                    Report("water", under, $"HeronBrushWaterView under water: fog {RenderSettings.fog}, {RenderSettings.fogMode}, density {RenderSettings.fogDensity}, colour {RenderSettings.fogColor}");
                    viewer.transform.position = water.transform.position + Vector3.up * 5;
                    Next(); break;
                }
                case 7:
                    if (Elapsed < .3) break;
                    Report("water", !RenderSettings.fog, $"above the water the view restores fog={RenderSettings.fog}");
                    Finish();
                    break;
            }
        }
        catch (Exception e) { Debug.LogException(e); Report("play mode", false, e.Message); Finish(); }
    }

    static void Finish()
    {
        EditorApplication.update -= PlayTick;
        var summary = new StringBuilder();
        foreach (var verdict in Verdicts) summary.AppendLine($"{verdict.Key}: {(verdict.Value ? "PASS" : "FAIL")}");
        File.WriteAllText(Path.Combine(Project, "feature-lab-unity.txt"), string.Join("\n", Lines) + "\n\n" + summary);
        bool pass = Verdicts.Values.All(v => v);
        Debug.Log((pass ? "HERON_FEATURE_LAB PASS\n" : "HERON_FEATURE_LAB FAIL\n") + summary);
        EditorApplication.Exit(pass ? 0 : 1);
    }

    // ------------------------------------------------------------------ captures

    // Needs a GPU (no -nographics). Draws each station from the front, lit by
    // the exported lights and a fixed ambient, and writes captures/*.png.
    public static void Render()
    {
        try
        {
            EditorSceneManager.OpenScene(Scene);
            var stations = Stations(Root());
            // HeronBrushWater swaps in its shader on Start, which edit mode skips.
            typeof(HeronBrushWater).GetMethod("Start", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)
                .Invoke(stations["fl_water"].GetComponent<HeronBrushWater>(), null);
            var folder = Path.Combine(Project, "captures");
            Directory.CreateDirectory(folder);
            RenderSettings.ambientMode = UnityEngine.Rendering.AmbientMode.Flat;
            RenderSettings.ambientLight = new Color(.55f, .55f, .55f);
            var sun = new GameObject("Capture sun").AddComponent<Light>();
            sun.type = LightType.Directional; sun.intensity = .9f; sun.transform.rotation = Quaternion.Euler(50, 150, 0);
            var camera = new GameObject("Capture camera").AddComponent<Camera>();
            camera.clearFlags = CameraClearFlags.SolidColor; camera.backgroundColor = new Color(.12f, .16f, .22f); camera.fieldOfView = 50;
            var target = new RenderTexture(640, 480, 24); camera.targetTexture = target;
            var image = new Texture2D(640, 480, TextureFormat.RGB24, false);
            int distinct = 0;
            void Shoot(string name, Vector3 look, Vector3 from)
            {
                camera.transform.position = from; camera.transform.LookAt(look);
                camera.Render(); RenderTexture.active = target;
                image.ReadPixels(new Rect(0, 0, 640, 480), 0, 0); image.Apply(); RenderTexture.active = null;
                File.WriteAllBytes(Path.Combine(folder, name + ".png"), image.EncodeToPNG());
                distinct = Math.Max(distinct, image.GetPixels32().Select(c => (c.r, c.g, c.b)).Distinct().Count());
            }
            foreach (var name in new[] { "fl_textures", "fl_colours", "fl_gradients", "fl_surface", "fl_normal", "fl_skip", "fl_hidden", "fl_water" })
            {
                var centre = stations[name].GetComponentsInChildren<Renderer>().Select(r => r.bounds.center).Aggregate((a, b) => (a + b) / 2);
                Shoot(name + "_a", centre, centre + new Vector3(3.2f, 2.4f, 3.2f));
                Shoot(name + "_b", centre, centre + new Vector3(-3.2f, -1.2f, -3.2f));
            }
            Shoot("overview", new Vector3(0, 0, -12), new Vector3(10, 14, 8));
            Debug.Log($"HERON_FEATURE_RENDER {(distinct > 100 ? "PASS" : "FAIL")}: {folder}, up to {distinct} colours per capture");
            EditorApplication.Exit(distinct > 100 ? 0 : 1);
        }
        catch (Exception e) { Debug.LogException(e); EditorApplication.Exit(2); }
    }

    // ------------------------------------------------------------------ JSON

    // JsonUtility reads no maps or mixed arrays; this reads both.
    static class Json
    {
        public static object Parse(string text) { int i = 0; var value = Value(text, ref i); return value; }

        static void Space(string s, ref int i) { while (i < s.Length && char.IsWhiteSpace(s[i])) i++; }

        static object Value(string s, ref int i)
        {
            Space(s, ref i);
            switch (s[i])
            {
                case '{':
                {
                    var map = new Dictionary<string, object>(); i++; Space(s, ref i);
                    if (s[i] == '}') { i++; return map; }
                    while (true)
                    {
                        Space(s, ref i); var key = (string)Value(s, ref i); Space(s, ref i); i++;
                        map[key] = Value(s, ref i); Space(s, ref i);
                        if (s[i++] == '}') return map;
                    }
                }
                case '[':
                {
                    var list = new List<object>(); i++; Space(s, ref i);
                    if (s[i] == ']') { i++; return list; }
                    while (true) { list.Add(Value(s, ref i)); Space(s, ref i); if (s[i++] == ']') return list; }
                }
                case '"':
                {
                    var text = new StringBuilder(); i++;
                    while (s[i] != '"')
                    {
                        if (s[i] == '\\')
                        {
                            i++;
                            switch (s[i])
                            {
                                case 'n': text.Append('\n'); break;
                                case 't': text.Append('\t'); break;
                                case 'r': text.Append('\r'); break;
                                case 'u': text.Append((char)Convert.ToInt32(s.Substring(i + 1, 4), 16)); i += 4; break;
                                default: text.Append(s[i]); break;
                            }
                        }
                        else text.Append(s[i]);
                        i++;
                    }
                    i++; return text.ToString();
                }
                case 't': i += 4; return true;
                case 'f': i += 5; return false;
                case 'n': i += 4; return null;
                default:
                {
                    int start = i;
                    while (i < s.Length && "+-.eE0123456789".IndexOf(s[i]) >= 0) i++;
                    var number = s.Substring(start, i - start);
                    if (number.IndexOfAny(new[] { '.', 'e', 'E' }) < 0 && long.TryParse(number, NumberStyles.Integer, CultureInfo.InvariantCulture, out var whole)) return whole;
                    return double.Parse(number, CultureInfo.InvariantCulture);
                }
            }
        }

        public static string Write(object v)
        {
            switch (v)
            {
                case null: return "null";
                case string s: return "\"" + s.Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"";
                case bool b: return b ? "true" : "false";
                case long l: return l.ToString(CultureInfo.InvariantCulture);
                case double d: return d.ToString("R", CultureInfo.InvariantCulture);
                case List<object> list: return "[" + string.Join(",", list.Select(Write)) + "]";
                case Dictionary<string, object> map: return "{" + string.Join(",", map.Select(p => Write(p.Key) + ":" + Write(p.Value))) + "}";
                default: return v.ToString();
            }
        }
    }
}
