#if FLAX_EDITOR
        private sealed class MaterialOwner : FlaxEditor.Surface.IVisjectSurfaceOwner
        {
            public Asset SurfaceAsset { get; set; }
            public byte[] SurfaceData { get; set; }
            public string SurfaceName => "Heron material";
            public FlaxEditor.Surface.VisjectSurfaceContext ParentContext => null;
            public FlaxEditor.Undo Undo => null;
            public void OnContextCreated(FlaxEditor.Surface.VisjectSurfaceContext context) { }
            public void OnSurfaceEditedChanged() { }
            public void OnSurfaceGraphEdited() { }
            public void OnSurfaceClose() { }
        }
        private static void ApplyMaterialCode(FlaxEditor.Surface.VisjectSurfaceContext context, FlaxEditor.Surface.Elements.Box target, string code)
        {
            var source = target.Connections[0]; target.RemoveConnections(0);
            var node = context.SpawnNode(1, 8, new Float2(-200, 1200), new object[] { code, new Float2(350, 200) });
            source.CreateConnection(node.GetBox(0)); node.GetBox(8).CreateConnection(target);
        }
        private static bool ClampMaterialTexture(JObject gltf, JToken textureInfo, FlaxEditor.Surface.MaterialSurface surface, FlaxEditor.Surface.Elements.Box target)
        {
            var texture = gltf["textures"][(int)textureInfo["index"]];
            if (texture["sampler"] == null) return false;
            var sampler = gltf["samplers"][(int)texture["sampler"]];
            bool clampU = (int?)sampler["wrapS"] == 33071, clampV = (int?)sampler["wrapT"] == 33071;
            if (!clampU && !clampV) return false;
            var nodes = new HashSet<FlaxEditor.Surface.SurfaceNode>();
            var pending = new Stack<FlaxEditor.Surface.Elements.Box>(); pending.Push(target);
            while (pending.Count > 0)
                foreach (var connection in pending.Pop().Connections)
                {
                    var node = connection.ParentNode;
                    if (!nodes.Add(node)) continue;
                    for (int i = 0; i < 16; i++) { var box = node.GetBox(i); if (box != null && !box.IsOutput) pending.Push(box); }
                }
            foreach (var node in nodes)
            {
                if (node.GroupArchetype.GroupID != 5 || node.Archetype.TypeID != 1) continue;
                var context = surface.Context;
                // LinearClamp = 0; LinearWrap = 2 in Flax's texture sampler archetype.
                var sample = context.SpawnNode(5, 9, new Float2(-1000, 900), new object[] { clampU && clampV ? 0 : 2, -1f, 0 });
                node.GetBox(6).CreateConnection(sample.GetBox(0));
                var uv = context.SpawnNode(5, 2, new Float2(-1400, 900), new object[] { (uint?)textureInfo["texCoord"] ?? 0u });
                if (clampU != clampV)
                {
                    var fix = context.SpawnNode(1, 8, new Float2(-1200, 900), new object[] {
                        "Output0 = float4(" + (clampU ? "clamp(Input0.x, 0.00001, 0.99999)" : "Input0.x") + "," + (clampV ? "clamp(Input0.y, 0.00001, 0.99999)" : "Input0.y") + ",0,0);", new Float2(350, 200) });
                    uv.GetBox(0).CreateConnection(fix.GetBox(0)); fix.GetBox(8).CreateConnection(sample.GetBox(1));
                }
                else uv.GetBox(0).CreateConnection(sample.GetBox(1));
                for (int channel = 1; channel <= 5; channel++)
                {
                    var output = node.GetBox(channel);
                    if (output == null || output.Connections.Count == 0) continue;
                    FlaxEditor.Surface.Elements.Box replacement = sample.GetBox(4);
                    if (channel > 1)
                    {
                        var split = context.SpawnNode(1, 8, new Float2(-700, 900 + channel * 100), new object[] {
                            "Output0 = Input0." + "rgba"[channel - 2] + ";", new Float2(350, 200) });
                        replacement.CreateConnection(split.GetBox(0)); replacement = split.GetBox(8);
                    }
                    foreach (var destination in output.Connections.ToArray()) { output.BreakConnection(destination); replacement.CreateConnection(destination); }
                }
            }
            return nodes.Count > 0;
        }
        // Flax's model importer omits glTF occlusion and sampler addressing.
        private static void RepairMaterials(string path)
        {
            JObject gltf;
            byte[] binary = null;
            using (var reader = new BinaryReader(File.OpenRead(path + ".glb")))
            {
                if (reader.ReadUInt32() != 0x46546c67 || reader.ReadUInt32() != 2)
                    throw new InvalidOperationException("Invalid GLB: " + path);
                uint length = reader.ReadUInt32();
                gltf = null;
                while (reader.BaseStream.Position < length)
                {
                    int size = checked((int)reader.ReadUInt32());
                    uint type = reader.ReadUInt32();
                    var bytes = reader.ReadBytes(size);
                    if (bytes.Length != size) throw new EndOfStreamException();
                    if (type == 0x4e4f534a) gltf = JObject.Parse(Encoding.UTF8.GetString(bytes));
                    if (type == 0x004e4942) binary = bytes;
                }
            }
            if (gltf == null || !(gltf["materials"] is JArray materials)) return;
            var model = Content.Load<Model>(path + ".flax");
            if (model == null || model.WaitForLoaded()) throw new InvalidOperationException("Model unavailable: " + path);
            foreach (var slot in model.MaterialSlots)
            {
                JObject definition = null;
                foreach (JObject candidate in materials)
                    if ((string)candidate["name"] == slot.Name) { definition = candidate; break; }
                if (definition == null || !(slot.Material is Material material)) continue;
                if (material.WaitForLoaded()) throw new InvalidOperationException("Material unavailable: " + slot.Name);
                var owner = new MaterialOwner { SurfaceAsset = material, SurfaceData = material.LoadSurface(false) };
                var surface = new FlaxEditor.Surface.MaterialSurface(owner);
                try
                {
                    if (surface.Load()) throw new InvalidOperationException("Cannot load material graph: " + slot.Name);
                    var context = surface.Context;
                    var root = context.FindNode(1, 1);
                    bool edited = false;
                    if (definition["occlusionTexture"] is JObject occlusion)
                    {
                        int index = (int)occlusion["index"];
                        var texture = gltf["textures"][index];
                        var image = gltf["images"][(int)texture["source"]];
                        if (image["bufferView"] == null || binary == null)
                            throw new InvalidOperationException("Occlusion image must be embedded in GLB.");
                        var view = gltf["bufferViews"][(int)image["bufferView"]];
                        int offset = (int?)view["byteOffset"] ?? 0, count = (int)view["byteLength"];
                        if ((int?)view["buffer"] != 0 || offset < 0 || count < 0 || offset > binary.Length - count)
                            throw new InvalidOperationException("Invalid occlusion image range.");
                        var bytes = new byte[count]; Array.Copy(binary, offset, bytes, 0, count);
                        string hash;
                        using (var sha = SHA256.Create()) hash = BitConverter.ToString(sha.ComputeHash(bytes)).Replace("-", "").ToLowerInvariant();
                        string imagePath = Path.Combine(Path.GetDirectoryName(path), "HeronTextures", hash + "_ao");
                        Directory.CreateDirectory(Path.GetDirectoryName(imagePath));
                        string source = imagePath + ((string)image["mimeType"] == "image/jpeg" ? ".jpg" : ".png");
                        File.WriteAllBytes(source, bytes);
                        if (!File.Exists(imagePath + ".flax"))
                        {
                            var options = new FlaxEngine.Tools.TextureTool.Options { Type = TextureFormatType.ColorRGB, sRGB = false, Compress = false, GenerateMipMaps = true, NeverStream = true, Scale = 1f, MaxSize = 8192 };
                            if (FlaxEditor.Editor.Import(source, imagePath + ".flax", options)) throw new InvalidOperationException("Occlusion import failed.");
                        }
                        var asset = Content.Load<Texture>(imagePath + ".flax");
                        if (asset == null || asset.WaitForLoaded()) throw new InvalidOperationException("Occlusion texture unavailable.");
                        var node = context.SpawnNode(5, 1, new Float2(-500, 700), new object[] { asset.ID });
                        var factor = context.SpawnNode(1, 8, new Float2(-200, 700), new object[] {
                            "Output0 = lerp(1.0, Input0.x, " + ((float?)occlusion["strength"] ?? 1f).ToString("R", System.Globalization.CultureInfo.InvariantCulture) + ");", new Float2(350, 200) });
                        node.GetBox(2).CreateConnection(factor.GetBox(0));
                        root.GetBox(7).RemoveConnections(0);
                        factor.GetBox(8).CreateConnection(root.GetBox(7));
                        edited = true;
                    }
                    float emission = (float?)definition["extensions"]?["KHR_materials_emissive_strength"]?["emissiveStrength"] ?? 1f;
                    if (emission != 1f && root.GetBox(3).Connections.Count > 0)
                    {
                        ApplyMaterialCode(context, root.GetBox(3), "Output0 = Input0 * " + emission.ToString("R", System.Globalization.CultureInfo.InvariantCulture) + ";");
                        edited = true;
                    }
                    float normalScale = (float?)definition["normalTexture"]?["scale"] ?? 1f;
                    if (normalScale != 1f && root.GetBox(8).Connections.Count > 0)
                    {
                        ApplyMaterialCode(context, root.GetBox(8), "Output0 = float4(normalize(float3(Input0.xy * " + normalScale.ToString("R", System.Globalization.CultureInfo.InvariantCulture) + ", Input0.z)), 0);");
                        edited = true;
                    }
                    if (definition["extensions"]?["KHR_materials_clearcoat"] != null || definition["extensions"]?["KHR_materials_anisotropy"] != null)
                        Debug.LogWarning("Heron Brush: Flax Lit materials do not support clear coat or anisotropy: " + slot.Name);
                    var colorTexture = definition["pbrMetallicRoughness"]?["baseColorTexture"];
                    if (colorTexture != null) edited |= ClampMaterialTexture(gltf, colorTexture, surface, root.GetBox(1));
                    if (definition["occlusionTexture"] != null) edited |= ClampMaterialTexture(gltf, definition["occlusionTexture"], surface, root.GetBox(7));
                    if (edited)
                    {
                        if (context.Save()) throw new InvalidOperationException("Cannot serialize material graph.");
                        if (material.SaveSurface(owner.SurfaceData, material.Info)) throw new InvalidOperationException("Cannot save material graph.");
                    }
                }
                finally { surface.Dispose(); }
            }
        }
#endif
