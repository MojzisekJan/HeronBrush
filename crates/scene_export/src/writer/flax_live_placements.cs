private static Placement[] Placements = Array.Empty<Placement>();
        public static void LoadSnapshot(string path)
        {
            var snapshot = JObject.Parse(File.ReadAllText(path));
            if ((int?)snapshot["version"] != 1) throw new InvalidOperationException("Unsupported Heron Brush live snapshot.");
            var result = new List<Placement>();
            foreach (var p in (JArray)snapshot["placements"])
            {
                var position = (JArray)p["position"];
                var orientation = (JArray)p["orientation"];
                var scale = (JArray)p["scale"];
                string mesh = (string)p["mesh"];
                if (mesh != null && (!mesh.StartsWith("HeronBrushLive/preview/", StringComparison.Ordinal) || mesh.Contains("..") || mesh.Contains("\\")))
                    throw new InvalidOperationException("Mesh outside the live preview folder.");
                result.Add(new Placement((string)p["name"], (string)p["classname"], mesh,
                    new Vector3((float)position[0], (float)position[1], (float)position[2]),
                    new Quaternion((float)orientation[0], (float)orientation[1], (float)orientation[2], (float)orientation[3]),
                    new Vector3((float)scale[0], (float)scale[1], (float)scale[2]),
                    p["metadata"].ToString(Newtonsoft.Json.Formatting.None)));
            }
            Placements = result.ToArray();
        }
