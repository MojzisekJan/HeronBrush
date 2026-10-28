    private static void ApplyEnvironment(GameObject root, string here)
    {
        var panoramaPath = ToAssetPath(Path.Combine(here, "__LEVEL__/sky.hdr"));
        var importer = AssetImporter.GetAtPath(panoramaPath) as TextureImporter;
        if (importer == null) throw new System.InvalidOperationException("Heron Brush: missing sky " + panoramaPath);
        if (importer.sRGBTexture || importer.textureCompression != TextureImporterCompression.Uncompressed || importer.wrapModeU != TextureWrapMode.Repeat || importer.wrapModeV != TextureWrapMode.Clamp)
        {
            importer.sRGBTexture = false;
            importer.textureCompression = TextureImporterCompression.Uncompressed;
            importer.wrapModeU = TextureWrapMode.Repeat;
            importer.wrapModeV = TextureWrapMode.Clamp;
            importer.SaveAndReimport();
        }
        var panorama = AssetDatabase.LoadAssetAtPath<Texture2D>(panoramaPath);
        var shader = Shader.Find("Skybox/Panoramic");
        if (panorama == null || shader == null) throw new System.InvalidOperationException("Heron Brush: panoramic sky resources unavailable");
        var folder = Path.Combine(here, "Materials");
        Directory.CreateDirectory(folder); AssetDatabase.Refresh();
        var path = ToAssetPath(Path.Combine(folder, "HeronSky.mat"));
        var material = AssetDatabase.LoadAssetAtPath<Material>(path);
        bool create = material == null;
        if (create) material = new Material(shader); else material.shader = shader;
        material.SetTexture("_MainTex", panorama);
        material.SetColor("_Tint", new Color(0.5f,0.5f,0.5f,1f));
        material.SetFloat("_Exposure", __EXPOSURE__f);
        material.SetFloat("_Mapping", 1f); material.SetFloat("_ImageType", 0f); material.SetFloat("_Rotation", 0f);
        material.DisableKeyword("_MAPPING_6_FRAMES_LAYOUT");
        if (create) AssetDatabase.CreateAsset(material,path); else EditorUtility.SetDirty(material);
        Light sun = null;
        foreach (var light in root.GetComponentsInChildren<Light>(true)) if (light.type == LightType.Directional) { sun = light; break; }
        if (sun == null)
        {
            var node = new GameObject("HeronSun"); node.transform.SetParent(root.transform,false);
            sun = node.AddComponent<Light>(); sun.type = LightType.Directional;
            sun.color = new Color(__SUN_COLOR__,1f);
            sun.intensity = __SUN_INTENSITY__f; sun.shadows = LightShadows.Soft;
            var direction = new Vector3(__SUN_DIRECTION__);
            sun.transform.localRotation = Quaternion.LookRotation(direction, Mathf.Abs(direction.normalized.y) > 0.999f ? Vector3.forward : Vector3.up);
        }
        RenderSettings.skybox = material;
        RenderSettings.sun = sun;
        RenderSettings.ambientMode = UnityEngine.Rendering.AmbientMode.Skybox;
        RenderSettings.ambientIntensity = __AMBIENT__f;
        RenderSettings.defaultReflectionMode = UnityEngine.Rendering.DefaultReflectionMode.Skybox;
        RenderSettings.fog = __FOG_ENABLED__;
        RenderSettings.fogMode = FogMode.ExponentialSquared;
        RenderSettings.fogColor = new Color(__FOG_COLOR__,1f);
        RenderSettings.fogDensity = __FOG_DENSITY__f;
        if (SystemInfo.graphicsDeviceType != UnityEngine.Rendering.GraphicsDeviceType.Null) DynamicGI.UpdateEnvironment();
    }
