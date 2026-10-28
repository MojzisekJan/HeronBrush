        private static void AddLight(Actor actor, JObject data)
        {
            if (!(data["light"] is JObject spec)) return;
            LightWithShadow light;
            var kind = (string)spec["kind"];
            if (kind == "Point") light = new PointLight { Radius = (float)spec["range"] * 100f };
            else if (kind == "Spot") light = new SpotLight { Radius = (float)spec["range"] * 100f, OuterConeAngle = (float)spec["half_angle"], InnerConeAngle = 0f };
            else if (kind == "Directional") light = new DirectionalLight();
            else return;
            var color = (JArray)spec["color"];
            light.Color = new Color((float)color[0], (float)color[1], (float)color[2], (float)color[3]);
            light.Brightness = (float)spec["energy"];
            light.ShadowsMode = (bool)spec["shadows"] ? ShadowsCastingMode.All : ShadowsCastingMode.None;
            light.Name = "HeronLight";
            if (Level.SpawnActor(light, actor)) throw new InvalidOperationException("Cannot spawn light for " + actor.Name);
            // Z reflection maps the source -Z direction to Flax's local +Z.
            light.LocalTransform = Transform.Identity;
        }
