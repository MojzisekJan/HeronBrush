    // glTF uses MR.G for roughness and MR.B for metallic. Unity's Standard and
    // URP Lit use metallic.R and smoothness.A. Derived assets keep this conversion
    // explicit rather than depending on Unity's interpretation of OBJ MTL extras.
    private static Texture2D PbrImage(string here, string directory, string name, string slot, bool srgb, bool normal = false)
    {
        var path = ToAssetPath(Path.Combine(here, directory, name + "_" + slot + ".png"));
        var importer = AssetImporter.GetAtPath(path) as TextureImporter;
        if (importer == null) throw new System.InvalidOperationException("Heron Brush: missing PBR image " + path);
        var type = normal ? TextureImporterType.NormalMap : TextureImporterType.Default;
        if (importer.sRGBTexture != srgb || importer.textureType != type || !importer.isReadable || importer.textureCompression != TextureImporterCompression.Uncompressed || importer.npotScale != TextureImporterNPOTScale.None || importer.maxTextureSize != 16384)
        {
            importer.sRGBTexture = srgb;
            importer.textureType = type;
            importer.isReadable = true;
            importer.npotScale = TextureImporterNPOTScale.None;
            importer.maxTextureSize = 16384;
            importer.textureCompression = TextureImporterCompression.Uncompressed;
            importer.SaveAndReimport();
        }
        return AssetDatabase.LoadAssetAtPath<Texture2D>(path);
    }

    private static Texture2D PbrDerived(string here, string name, string slot, Texture2D source, System.Func<Color, Color> convert)
    {
        var directory = Path.Combine(here, "Materials");
        Directory.CreateDirectory(directory);
        var path = ToAssetPath(Path.Combine(directory, name + "_" + slot + ".png"));
        var pixels = source.GetPixels();
        for (int i = 0; i < pixels.Length; i++) pixels[i] = convert(pixels[i]);
        var image = new Texture2D(source.width, source.height, TextureFormat.RGBA32, false, true);
        try
        {
            image.SetPixels(pixels);
            image.Apply();
            File.WriteAllBytes(path, image.EncodeToPNG());
        }
        finally { Object.DestroyImmediate(image); }
        AssetDatabase.ImportAsset(path, ImportAssetOptions.ForceSynchronousImport);
        var importer = (TextureImporter)AssetImporter.GetAtPath(path);
        importer.sRGBTexture = false;
        importer.npotScale = TextureImporterNPOTScale.None;
        importer.maxTextureSize = 16384;
        importer.textureCompression = TextureImporterCompression.Uncompressed;
        importer.SaveAndReimport();
        return AssetDatabase.LoadAssetAtPath<Texture2D>(path);
    }

    private static void ApplyPbr(GameObject root, string here, string name, string directory,
        float roughness, float metallic, Color emission, float normalScale, float occlusionStrength,
        bool hasMr, bool hasNormal, bool hasAo, bool hasEmission, int alphaMode, float alphaCutoff, bool doubleSided)
    {
        bool alpha = alphaMode == 2, masked = alphaMode == 1;
        var pipeline = UnityEngine.Rendering.GraphicsSettings.currentRenderPipeline;
        bool urp = pipeline != null;
        if (urp && !pipeline.GetType().Name.Contains("Universal"))
            throw new System.InvalidOperationException("Heron Brush: configure this render pipeline's PBR materials explicitly; only Standard and URP Lit are supported by this adapter.");
        var shader = Shader.Find(urp ? "Universal Render Pipeline/Lit" : doubleSided ? "Heron Brush/Standard Two Sided" : "Standard");
        if (shader == null) throw new System.InvalidOperationException("Heron Brush: PBR needs Standard or URP Lit; configure another pipeline's materials explicitly.");
        Texture2D mr = hasMr ? PbrImage(here, directory, name, "mr", false) : null;
        Texture2D mask = mr == null ? null : PbrDerived(here, name, "metallic", mr,
            c => new Color(c.b * metallic, 0f, 0f, 1f - c.g * roughness));
        Texture2D ao = hasAo ? PbrImage(here, directory, name, "ao", false) : null;
        Texture2D aoMap = ao == null ? null : PbrDerived(here, name, "occlusion", ao,
            c => new Color(1f, Mathf.Lerp(1f, c.r, occlusionStrength), 1f, 1f));
        var normal = hasNormal ? PbrImage(here, directory, name, "normal", false, true) : null;
        var emissive = hasEmission ? PbrImage(here, directory, name, "emissive", true) : null;
        Material replacement = null;
        foreach (var renderer in root.GetComponentsInChildren<Renderer>(true))
        {
            var slots = renderer.sharedMaterials;
            for (int i = 0; i < slots.Length; i++)
            {
                var source = slots[i];
                if (source == null || source.name != name) continue;
                if (replacement == null)
                {
                    var materialPath = ToAssetPath(Path.Combine(here, "Materials", name + ".mat"));
                    Directory.CreateDirectory(Path.GetDirectoryName(materialPath));
                    AssetDatabase.Refresh();
                    replacement = AssetDatabase.LoadAssetAtPath<Material>(materialPath);
                    bool create = replacement == null;
                    if (create) replacement = new Material(shader);
                    else replacement.shader = shader;
                    replacement.name = name;
                    if (replacement.HasProperty("_Cull")) replacement.SetFloat("_Cull", doubleSided ? 0f : 2f);
                    replacement.doubleSidedGI = doubleSided;
                    replacement.mainTexture = source.mainTexture;
                    replacement.SetColor(urp ? "_BaseColor" : "_Color", Color.white);
                    replacement.SetFloat("_Metallic", mask == null ? metallic : 1f);
                    replacement.SetFloat(urp ? "_Smoothness" : "_Glossiness", mask == null ? 1f - roughness : 1f);
                    if (replacement.HasProperty("_GlossMapScale")) replacement.SetFloat("_GlossMapScale", 1f);
                    replacement.SetTexture("_MetallicGlossMap", mask);
                    if (mask != null) replacement.EnableKeyword("_METALLICGLOSSMAP"); else replacement.DisableKeyword("_METALLICGLOSSMAP");
                    replacement.SetTexture("_BumpMap", normal);
                    replacement.SetFloat("_BumpScale", normalScale);
                    if (normal != null) replacement.EnableKeyword("_NORMALMAP"); else replacement.DisableKeyword("_NORMALMAP");
                    replacement.SetTexture("_OcclusionMap", aoMap);
                    replacement.SetFloat("_OcclusionStrength", 1f);
                    if (urp && aoMap != null) replacement.EnableKeyword("_OCCLUSIONMAP"); else replacement.DisableKeyword("_OCCLUSIONMAP");
                    replacement.SetTexture("_EmissionMap", emissive);
                    replacement.SetColor("_EmissionColor", emission);
                    replacement.globalIlluminationFlags = emission.maxColorComponent > 0f
                        ? MaterialGlobalIlluminationFlags.BakedEmissive : MaterialGlobalIlluminationFlags.EmissiveIsBlack;
                    if (emission.maxColorComponent > 0f) replacement.EnableKeyword("_EMISSION"); else replacement.DisableKeyword("_EMISSION");
                    replacement.SetOverrideTag("RenderType", alpha ? "Transparent" : masked ? "TransparentCutout" : "Opaque");
                    replacement.SetFloat("_Cutoff", alphaCutoff);
                    if (urp) replacement.SetFloat("_AlphaClip", masked ? 1f : 0f);
                    replacement.DisableKeyword("_ALPHATEST_ON");
                    replacement.DisableKeyword("_ALPHAPREMULTIPLY_ON");
                    replacement.DisableKeyword("_ALPHABLEND_ON");
                    replacement.DisableKeyword("_SURFACE_TYPE_TRANSPARENT");
                    if (masked) replacement.EnableKeyword("_ALPHATEST_ON");
                    replacement.SetFloat(urp ? "_Surface" : "_Mode", alpha ? (urp ? 1f : 2f) : masked && !urp ? 1f : 0f);
                    replacement.SetFloat("_SrcBlend", alpha ? (float)UnityEngine.Rendering.BlendMode.SrcAlpha : (float)UnityEngine.Rendering.BlendMode.One);
                    replacement.SetFloat("_DstBlend", alpha ? (float)UnityEngine.Rendering.BlendMode.OneMinusSrcAlpha : (float)UnityEngine.Rendering.BlendMode.Zero);
                    replacement.SetFloat("_ZWrite", alpha ? 0f : 1f);
                    if (alpha) { replacement.EnableKeyword(urp ? "_SURFACE_TYPE_TRANSPARENT" : "_ALPHABLEND_ON"); }
                    else { replacement.DisableKeyword("_SURFACE_TYPE_TRANSPARENT"); replacement.DisableKeyword("_ALPHABLEND_ON"); }
                    replacement.renderQueue = alpha ? 3000 : masked ? 2450 : -1;
                    if (create) AssetDatabase.CreateAsset(replacement, materialPath); else EditorUtility.SetDirty(replacement);
                }
                slots[i] = replacement;
            }
            renderer.sharedMaterials = slots;
        }
    }

    // A brush texture's normal map: <name>_normal.png beside the albedo, already in
    // Unity's OpenGL convention. Reuses the surface material when ApplySurface made one.
    private static void ApplyNormal(GameObject root, string here, string name, string directory)
    {
        var normal = PbrImage(here, directory, name, "normal", false, true);
        var path = ToAssetPath(Path.Combine(here, "Materials", name + ".mat"));
        Material replacement = null;
        foreach (var renderer in root.GetComponentsInChildren<Renderer>(true))
        {
            var slots = renderer.sharedMaterials;
            for (int i = 0; i < slots.Length; i++)
            {
                var source = slots[i];
                if (source == null || source.name != name) continue;
                if (replacement == null)
                {
                    replacement = AssetDatabase.LoadAssetAtPath<Material>(path);
                    bool create = replacement == null;
                    if (create) replacement = new Material(source);
                    else if (replacement != source) replacement.CopyPropertiesFromMaterial(source);
                    if (replacement != source)
                    {
                        var shader = UnityEngine.Rendering.GraphicsSettings.currentRenderPipeline == null
                            ? Shader.Find("Standard") : Shader.Find("Universal Render Pipeline/Lit");
                        if (shader != null) replacement.shader = shader;
                        replacement.name = name;
                    }
                    replacement.SetTexture("_BumpMap", normal);
                    replacement.SetFloat("_BumpScale", 1f);
                    replacement.EnableKeyword("_NORMALMAP");
                    if (create)
                    {
                        Directory.CreateDirectory(Path.GetDirectoryName(path));
                        AssetDatabase.Refresh();
                        AssetDatabase.CreateAsset(replacement, path);
                    }
                    else EditorUtility.SetDirty(replacement);
                }
                slots[i] = replacement;
            }
            renderer.sharedMaterials = slots;
        }
    }

    // Generated paint images clamp, as in the other targets.
    private static void ClampPaint(string here, string directory, string[] names)
    {
        foreach (var name in names)
        {
            var importer = AssetImporter.GetAtPath(ToAssetPath(Path.Combine(here, directory, name + ".png"))) as TextureImporter;
            if (importer == null || importer.wrapMode == TextureWrapMode.Clamp) continue;
            importer.wrapMode = TextureWrapMode.Clamp;
            importer.SaveAndReimport();
        }
    }
