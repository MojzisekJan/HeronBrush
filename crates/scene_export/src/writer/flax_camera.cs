        private static void AddCameraSettings(Actor root, JObject settings)
        {
            if (settings == null) return;
            var volume = new PostFxVolume { Name = "HeronCameraSettings", IsBounded = false, Priority = 100 };
            var eye = EyeAdaptationSettings.Default;
            eye.OverrideFlags = EyeAdaptationSettingsOverride.All;
            eye.Mode = EyeAdaptationMode.None;
            eye.PreExposure = (float)Math.Log2(Mathf.Max((float)settings["exposure"] * 1.8f, 0.000001f));
            eye.PostExposure = 0f;
            volume.EyeAdaptation = eye;
            var tone = ToneMappingSettings.Default;
            tone.OverrideFlags = ToneMappingSettingsOverride.All;
            tone.WhiteTemperature = 6500f; tone.WhiteTint = 0f;
            switch ((string)settings["tone_map"])
            {
                case "aces": tone.Mode = ToneMappingMode.ACES; break;
                case "neutral": tone.Mode = ToneMappingMode.Neutral; break;
                default: tone.Mode = ToneMappingMode.AGX; break;
            }
            volume.ToneMapping = tone;
            var grading = ColorGradingSettings.Default;
            grading.OverrideFlags = ColorGradingSettingsOverride.ColorGain | ColorGradingSettingsOverride.ColorContrast | ColorGradingSettingsOverride.ColorSaturation;
            var gain = (JArray)settings["white_balance"];
            grading.ColorGain = new Float4((float)gain[0], (float)gain[1], (float)gain[2], 1f);
            grading.ColorContrast = new Float4(1f, 1f, 1f, (float)settings["contrast"]);
            grading.ColorSaturation = new Float4(1f, 1f, 1f, (float)settings["saturation"]);
            volume.ColorGrading = grading;
            if (Level.SpawnActor(volume, root)) throw new InvalidOperationException("Cannot spawn camera settings.");
            volume.LocalTransform = Transform.Identity;
        }
