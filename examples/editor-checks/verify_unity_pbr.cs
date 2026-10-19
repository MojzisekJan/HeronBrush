#if UNITY_EDITOR
using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;

public static class VerifyHeronPbr
{
    static void Require(bool ok, string message) { if (!ok) throw new Exception(message); }
    static void Near(float actual, float expected, string message) { Require(Mathf.Abs(actual - expected) < .006f, message + ": " + actual); }
    public static void Run()
    {
        try
        {
            var importer = AppDomain.CurrentDomain.GetAssemblies().SelectMany(a => a.GetTypes())
                .Single(t => t.Name == "HeronBrushLevelImporter");
            importer.GetMethod("Build").Invoke(null, null);
            Check();
            var paths = AssetDatabase.FindAssets("t:Material", new[]{"Assets/Level/Materials"})
                .Select(AssetDatabase.GUIDToAssetPath).OrderBy(p => p).ToArray();
            Require(paths.Length == 2, "Expected two generated PBR materials");
            var guids = paths.Select(AssetDatabase.AssetPathToGUID).ToArray();
            importer.GetMethod("Build").Invoke(null, null);
            Check();
            Require(guids.SequenceEqual(paths.Select(AssetDatabase.AssetPathToGUID)), "Rebuild replaced material GUIDs");
            var scene = UnityEngine.SceneManagement.SceneManager.GetActiveScene();
            Require(EditorSceneManager.SaveScene(scene, "Assets/pbr_verified.unity"), "Scene save failed");
            EditorSceneManager.OpenScene("Assets/pbr_verified.unity", OpenSceneMode.Single);
            Check();
            Debug.Log("HERON_PBR_PASS: native Standard materials, MR/AO channels, normal import, emission, rebuild and scene reopen");
            EditorApplication.Exit(0);
        }
        catch (Exception error) { Debug.LogException(error); EditorApplication.Exit(1); }
    }
    static void Check()
    {
        var root = GameObject.Find("pbr");
        Require(root != null, "Missing level root");
        var materials = root.GetComponentsInChildren<Renderer>(true).SelectMany(r => r.sharedMaterials).Distinct().ToArray();
        Require(materials.Length == 2, "PBR surfaces lost distinct materials");
        var smoothness = new System.Collections.Generic.List<float>();
        foreach (var material in materials)
        {
            Require(material.shader.name == "Standard", "Wrong shader");
            Require(material.mainTexture != null, "Missing tinted base colour");
            Require(material.IsKeywordEnabled("_METALLICGLOSSMAP"), "Missing MR shader keyword");
            Require(material.IsKeywordEnabled("_NORMALMAP"), "Missing normal shader keyword");
            Require(material.IsKeywordEnabled("_EMISSION"), "Missing emission shader keyword");
            Near(material.GetFloat("_BumpScale"), .35f, "Normal strength");
            var emission = material.GetColor("_EmissionColor");
            Near(emission.r, .2f, "Emission R"); Near(emission.g, .4f, "Emission G"); Near(emission.b, .6f, "Emission B");
            var mr = (Texture2D)material.GetTexture("_MetallicGlossMap");
            var mrImporter = (TextureImporter)AssetImporter.GetAtPath(AssetDatabase.GetAssetPath(mr));
            Require(!mrImporter.sRGBTexture, "MR imported as sRGB");
            Require(mr.width == 2 && mr.height == 1, "MR dimensions changed");
            // Read the derived PNG independently of GPU/readability state.
            var image = new Texture2D(2, 1, TextureFormat.RGBA32, false, true);
            image.LoadImage(File.ReadAllBytes(AssetDatabase.GetAssetPath(mr)));
            var pixel = image.GetPixel(0, 0);
            Near(pixel.r, 192f / 255f * .4f, "MR blue-to-metallic red conversion");
            smoothness.Add(pixel.a);
            UnityEngine.Object.DestroyImmediate(image);
            var ao = (Texture2D)material.GetTexture("_OcclusionMap");
            Require(ao.width == 1 && ao.height == 2, "AO dimensions changed");
            image = new Texture2D(1, 2, TextureFormat.RGBA32, false, true);
            image.LoadImage(File.ReadAllBytes(AssetDatabase.GetAssetPath(ao)));
            Near(image.GetPixel(0,0).g, Mathf.Lerp(1f, 64f/255f, .25f), "AO red-to-green conversion and strength");
            UnityEngine.Object.DestroyImmediate(image);
            var normal = material.GetTexture("_BumpMap");
            var ni = (TextureImporter)AssetImporter.GetAtPath(AssetDatabase.GetAssetPath(normal));
            Require(ni.textureType == TextureImporterType.NormalMap && !ni.sRGBTexture, "Normal import settings");
            Require(material.GetTexture("_EmissionMap") != null, "Missing emission image");
        }
        smoothness.Sort();
        Near(smoothness[0], 1f - 128f/255f*.9f, "Roughness .9");
        Near(smoothness[1], 1f - 128f/255f*.6f, "Roughness .6");
    }
}
#endif
