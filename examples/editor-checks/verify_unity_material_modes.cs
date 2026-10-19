#if UNITY_EDITOR
using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEngine;
public static class VerifyHeronMaterialModes
{
    [Serializable] public class Expected { public string name; public string mode; public float cutoff; public bool doubleSided; }
    [Serializable] public class Expectations { public Expected[] materials; }
    static void Require(bool value, string message) { if (!value) throw new Exception(message); }
    static void Check()
    {
        var expected = JsonUtility.FromJson<Expectations>(File.ReadAllText("Assets/expected_material_modes.json"));
        var root = GameObject.Find("modes");
        Require(root != null, "Missing imported root");
        var actual = root.GetComponentsInChildren<Renderer>(true).SelectMany(r=>r.sharedMaterials).Distinct().ToArray();
        Require(actual.Length == 6 && expected.materials.Length == 6, "Expected six materials");
        foreach (var item in expected.materials)
        {
            var material = actual.Single(m=>m.name == item.name);
            bool mask = item.mode == "MASK", blend = item.mode == "BLEND";
            Require(material.shader.name == (item.doubleSided ? "Heron Brush/Standard Two Sided" : "Standard"), "Wrong shader");
            Require(!ShaderUtil.ShaderHasError(material.shader), "Shader compiler error: " + material.shader.name);
            Require(material.doubleSidedGI == item.doubleSided, item.name + " GI sidedness");
            if (item.doubleSided) Require(material.GetFloat("_Cull") == 0, item.name + " culling");
            Require(material.GetFloat("_Mode") == (blend ? 2 : mask ? 1 : 0), item.name + " mode");
            Require(material.IsKeywordEnabled("_ALPHATEST_ON") == mask, item.name + " mask keyword");
            Require(material.IsKeywordEnabled("_ALPHABLEND_ON") == blend, item.name + " blend keyword");
            Require(!material.IsKeywordEnabled("_ALPHAPREMULTIPLY_ON"), item.name + " premultiply keyword");
            Require(Math.Abs(material.GetFloat("_Cutoff") - item.cutoff) < .0001f, item.name + " cutoff");
            Require(material.GetFloat("_ZWrite") == (blend ? 0 : 1), item.name + " depth write");
            Require(material.renderQueue == (blend ? 3000 : mask ? 2450 : material.shader.renderQueue), item.name + " render queue");
        }
    }
    static float RenderCoverage(Material source, bool back, string label, out float partial)
    {
        var material = new Material(source);
        material.SetColor("_Color", new Color(0,0,0,1));
        material.SetColor("_EmissionColor", Color.white);
        material.SetTexture("_EmissionMap", Texture2D.whiteTexture);
        material.EnableKeyword("_EMISSION");
        var quad = GameObject.CreatePrimitive(PrimitiveType.Quad);
        quad.layer = 30; quad.transform.localScale = new Vector3(2,2,2);
        quad.GetComponent<Renderer>().sharedMaterial = material;
        var cameraObject = new GameObject("Material probe camera");
        var camera = cameraObject.AddComponent<Camera>();
        camera.transform.position = new Vector3(0,0,back ? 3 : -3);
        camera.transform.rotation = back ? Quaternion.Euler(0,180,0) : Quaternion.identity;
        camera.orthographic = true; camera.orthographicSize = 1.25f;
        camera.clearFlags = CameraClearFlags.SolidColor; camera.backgroundColor = Color.black;
        camera.cullingMask = 1 << 30; camera.renderingPath = RenderingPath.Forward;
        var target = new RenderTexture(128,128,24,RenderTextureFormat.ARGB32,RenderTextureReadWrite.Linear);
        var pixels = new Texture2D(128,128,TextureFormat.RGBA32,false,true);
        var previous = RenderTexture.active;
        try
        {
            camera.targetTexture = target; target.Create(); camera.Render();
            RenderTexture.active = target;
            pixels.ReadPixels(new Rect(0,0,128,128),0,0); pixels.Apply();
            var colors = pixels.GetPixels();
            float coverage = colors.Sum(c=>c.r) / colors.Length;
            partial = (float)colors.Count(c=>c.r > .05f && c.r < .95f) / colors.Length;
            File.WriteAllBytes("Assets/" + label + ".png", pixels.EncodeToPNG());
            return coverage;
        }
        finally
        {
            RenderTexture.active = previous; camera.targetTexture = null; target.Release();
            UnityEngine.Object.DestroyImmediate(target); UnityEngine.Object.DestroyImmediate(pixels);
            UnityEngine.Object.DestroyImmediate(cameraObject); UnityEngine.Object.DestroyImmediate(quad);
            UnityEngine.Object.DestroyImmediate(material);
        }
    }
    public static void RunRendered()
    {
        try
        {
            Require(SystemInfo.graphicsDeviceType != UnityEngine.Rendering.GraphicsDeviceType.Null, "No graphics device");
            var importer = AppDomain.CurrentDomain.GetAssemblies().SelectMany(a=>a.GetTypes()).Single(t=>t.Name=="HeronBrushLevelImporter");
            importer.GetMethod("Build").Invoke(null,null); Check();
            var expected = JsonUtility.FromJson<Expectations>(File.ReadAllText("Assets/expected_material_modes.json"));
            var root = GameObject.Find("modes");
            var actual = root.GetComponentsInChildren<Renderer>(true).SelectMany(r=>r.sharedMaterials).Distinct().ToArray();
            foreach(var item in expected.materials)
            {
                var material = actual.Single(m=>m.name == item.name);
                float front = RenderCoverage(material,false,item.mode + (item.doubleSided ? "_double" : "_single") + "_front", out float frontPartial);
                float back = RenderCoverage(material,true,item.mode + (item.doubleSided ? "_double" : "_single") + "_back", out float backPartial);
                Debug.Log("HERON_COVERAGE " + item.mode + " double=" + item.doubleSided + " front=" + front + " back=" + back + " partial=" + frontPartial);
                Require(front > .05f, item.name + " front missing");
                if(item.mode=="OPAQUE") Require(front > .55f, item.name + " opaque coverage lost");
                else Require(front < .5f, item.name + " alpha ignored");
                if(item.mode=="BLEND") Require(frontPartial > .1f, item.name + " continuous alpha missing");
                else Require(frontPartial < .005f, item.name + " opaque/mask has partial coverage");
                if(item.doubleSided) Require(Math.Abs(front-back) < .02f, item.name + " back coverage changed");
                else Require(back < .005f, item.name + " back was not culled");
                Require(!ShaderUtil.ShaderHasError(material.shader), "Shader compiler errors after render");
            }
            Debug.Log("HERON_MATERIAL_RENDER_PASS: native front/back and opaque/masked/blended coverage");
            EditorApplication.Exit(0);
        }
        catch(Exception error) { Debug.LogException(error); EditorApplication.Exit(1); }
    }
    public static void Run()
    {
        try
        {
            var importer = AppDomain.CurrentDomain.GetAssemblies().SelectMany(a=>a.GetTypes()).Single(t=>t.Name=="HeronBrushLevelImporter");
            importer.GetMethod("Build").Invoke(null,null);
            Check();
            importer.GetMethod("Build").Invoke(null,null);
            Check();
            Debug.Log("HERON_MATERIAL_MODES_PASS: native Standard modes, cutoff, keywords, depth writes, queues, two-sided shader/culling, GI and rebuild");
            EditorApplication.Exit(0);
        }
        catch(Exception error) { Debug.LogException(error); EditorApplication.Exit(1); }
    }
}
#endif
