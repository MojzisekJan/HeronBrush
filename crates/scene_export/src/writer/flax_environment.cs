#if FLAX_EDITOR
        private static void ImportEnvironment()
        {
            var path = Path.Combine(Globals.ProjectContentFolder, ContentDir, "sky");
            var options = new FlaxEngine.Tools.TextureTool.Options { Type = TextureFormatType.HdrRGB, sRGB = false, Compress = false, GenerateMipMaps = true, NeverStream = true, Scale = 1f, MaxSize = 4096 };
            if (FlaxEditor.Editor.Import(path + ".dds", path + ".flax", options)) throw new InvalidOperationException("Sky import failed: " + path);
        }
#endif
        private static void AddEnvironment(Actor root, Dictionary<string,Actor> actors)
        {
            var texture = Content.Load<CubeTexture>(Path.Combine(Globals.ProjectContentFolder, ContentDir, "sky.flax"));
            if (texture == null || texture.WaitForLoaded()) throw new InvalidOperationException("Heron Brush: sky cube unavailable");
            var sky = new Skybox { Name = "HeronSky", CubeTexture = texture, Exposure = __EXPOSURE__f };
            if (Level.SpawnActor(sky,root)) throw new InvalidOperationException("Cannot spawn sky");
            sky.LocalTransform = Transform.Identity;
            var ambient = new SkyLight { Name = "HeronAmbient", Mode = SkyLight.Modes.CustomTexture, CustomTexture = texture, Brightness = __AMBIENT__f };
            if (Level.SpawnActor(ambient,root)) throw new InvalidOperationException("Cannot spawn ambient light");
            ambient.LocalTransform = Transform.Identity;
            bool hasSun = false;
            foreach (var actor in actors.Values) if (actor.GetChild<DirectionalLight>() != null) { hasSun = true; break; }
            if (!hasSun)
            {
                var sun = new DirectionalLight { Name = "HeronSun", Color = new Color(__SUN_COLOR__,1f), Brightness = __SUN_INTENSITY__f, ShadowsMode = ShadowsCastingMode.All };
                if (Level.SpawnActor(sun,root)) throw new InvalidOperationException("Cannot spawn sun");
                var direction = new Vector3(__SUN_DIRECTION__);
                sun.LocalOrientation = Quaternion.LookRotation(direction, Mathf.Abs(direction.Y) > 0.999f ? Vector3.Forward : Vector3.Up);
            }
        }
