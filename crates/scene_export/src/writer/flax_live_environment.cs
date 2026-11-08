        private static JObject LiveEnvironment;
#if FLAX_EDITOR
        private static void ImportLiveEnvironment()
        {
            if (LiveEnvironment == null) return;
            ImportProbes(LiveEnvironment["probes"] as JArray);
            string source = (string)LiveEnvironment["sky"];
            if (source == null) return;
            if (!source.StartsWith("HeronBrushLive/preview/", StringComparison.Ordinal) || source.Contains("..") || source.Contains("\\"))
                throw new InvalidOperationException("Sky outside the live preview folder.");
            var path = Path.Combine(Globals.ProjectContentFolder, source);
            if (File.Exists(path + ".flax")) return;
            var options = new FlaxEngine.Tools.TextureTool.Options { Type = TextureFormatType.HdrRGB, sRGB = false, Compress = false, GenerateMipMaps = true, NeverStream = true, Scale = 1f, MaxSize = 4096 };
            if (FlaxEditor.Editor.Import(path, path + ".flax", options)) throw new InvalidOperationException("Sky import failed: " + path);
        }
#endif
        private static void AddLiveEnvironment(Actor root, Dictionary<string, Actor> actors)
        {
            var env = LiveEnvironment;
            if (env == null) return;
            var texture = Content.Load<CubeTexture>(Path.Combine(Globals.ProjectContentFolder, (string)env["sky"] + ".flax"));
            if (texture == null || texture.WaitForLoaded()) throw new InvalidOperationException("Live sky unavailable.");
            AddProbes(root, env["probes"] as JArray);
            AddCameraSettings(root, env["display"] as JObject);
            if (env["sky"] == null) return;
            float scale = (float)env["scale"];
            var sky = new Skybox { Name = "HeronSky", CubeTexture = texture, IsActive = (bool)env["show_background"], Exposure = (float)Math.Log2(Mathf.Max((env["display"] == null ? (float)env["exposure"] * 1.8f : 1f) * scale, 0.000001f)) };
            if (Level.SpawnActor(sky, root)) throw new InvalidOperationException("Cannot spawn sky.");
            sky.LocalTransform = Transform.Identity;
            var ambient = new SkyLight { Name = "HeronAmbient", Mode = SkyLight.Modes.CustomTexture, CustomTexture = texture, Brightness = (float)env["ambient"] * scale };
            if (Level.SpawnActor(ambient, root)) throw new InvalidOperationException("Cannot spawn ambient.");
            ambient.LocalTransform = Transform.Identity;
            if (env["fog"] is JArray fog)
            {
                var color = (JArray)fog[0];
                var actor = new ExponentialHeightFog { Name = "HeronFog", FogDensity = (float)fog[1] * 10f / (Mathf.Log(2f) * Mathf.Log(2f)), FogHeightFalloff = 0f, FogInscatteringColor = new Color((float)color[0], (float)color[1], (float)color[2], 1f) };
                if (Level.SpawnActor(actor, root)) throw new InvalidOperationException("Cannot spawn fog.");
                actor.LocalTransform = Transform.Identity;
            }
            foreach (var actor in actors.Values) if (actor.GetChild<DirectionalLight>() != null) return;
            var c = (JArray)env["sun_color"]; var d = (JArray)env["sun_direction"];
            var sun = new DirectionalLight { Name = "HeronSun", Color = new Color((float)c[0], (float)c[1], (float)c[2], 1f), Brightness = (float)env["sun_intensity"] / Mathf.Pi, ShadowsMode = (bool)env["shadows"] ? ShadowsCastingMode.All : ShadowsCastingMode.None };
            if (Level.SpawnActor(sun, root)) throw new InvalidOperationException("Cannot spawn sun.");
            var direction = new Vector3(-(float)d[0], -(float)d[1], (float)d[2]);
            sun.LocalOrientation = Quaternion.LookRotation(direction, Mathf.Abs(direction.Y) > 0.999f ? Vector3.Forward : Vector3.Up);
        }
