#if FLAX_EDITOR
        private static void ImportProbes(JArray probes)
        {
            if (probes == null) return;
            foreach (JObject probe in probes)
            {
                string source = (string)probe["texture"];
                if (string.IsNullOrEmpty(source) || source.Contains("..") || source.Contains("\\") || Path.IsPathRooted(source))
                    throw new InvalidOperationException("Invalid probe texture path.");
                var path = Path.Combine(Globals.ProjectContentFolder, source.Contains("/") ? source : ContentDir + "/" + source);
                if (File.Exists(path + ".flax")) continue;
                var options = new FlaxEngine.Tools.TextureTool.Options { Type = TextureFormatType.HdrRGB, sRGB = false, Compress = false, GenerateMipMaps = false, NeverStream = true, Scale = 1f, MaxSize = 4096 };
                if (FlaxEditor.Editor.Import(path, path + ".flax", options)) throw new InvalidOperationException("Probe import failed: " + path);
            }
        }
#endif
        private static void AddProbes(Actor root, JArray probes)
        {
            if (probes == null) return;
            bool diffuse = false; float distance = 1000f;
            foreach (JObject probe in probes)
            {
                var p = (JArray)probe["position"]; var h = (JArray)probe["half_size"];
                var size = new Vector3((float)h[0], (float)h[1], (float)h[2]);
                diffuse |= (bool)probe["diffuse"]; distance = Mathf.Max(distance, (float)size.Length * 2f);
                if (!(bool)probe["reflections"]) continue;
                string source = (string)probe["texture"];
                var texture = Content.Load<CubeTexture>(Path.Combine(Globals.ProjectContentFolder, (source.Contains("/") ? source : ContentDir + "/" + source) + ".flax"));
                if (texture == null || texture.WaitForLoaded()) throw new InvalidOperationException("Probe texture unavailable.");
                var actor = new EnvironmentProbe { Name = "HeronReflectionProbe", Radius = 1f, BoxProjection = true, CustomProbe = texture, Brightness = (float)probe["brightness"], BlendDistance = (float)probe["blend"] * Mathf.Min((float)size.X, Mathf.Min((float)size.Y, (float)size.Z)) };
                if (Level.SpawnActor(actor, root)) throw new InvalidOperationException("Cannot spawn probe.");
                actor.LocalTransform = new Transform(new Vector3((float)p[0], (float)p[1], (float)p[2]), Quaternion.Identity, size);
            }
            if (diffuse)
            {
                var volume = new PostFxVolume { Name = "HeronIndirectLighting", IsBounded = false, Priority = 100 };
                var gi = GlobalIlluminationSettings.Default;
                gi.OverrideFlags = GlobalIlluminationSettingsOverride.Mode | GlobalIlluminationSettingsOverride.Distance;
                gi.Mode = GlobalIlluminationMode.DDGI; gi.Distance = distance;
                volume.GlobalIllumination = gi;
                if (Level.SpawnActor(volume, root)) throw new InvalidOperationException("Cannot spawn indirect lighting settings.");
                volume.LocalTransform = Transform.Identity;
            }
        }
