private static Placement[] Placements = Array.Empty<Placement>();
        private static float SnapshotNumber(JArray values, int index)
        {
            float value=(float)values[index];
            if (float.IsNaN(value) || float.IsInfinity(value))
                throw new InvalidOperationException("Non-finite live transform.");
            return value;
        }
        public static void LoadSnapshot(string path)
        {
            var snapshot = JObject.Parse(File.ReadAllText(path));
            if ((int?)snapshot["version"] != 1) throw new InvalidOperationException("Unsupported Heron Brush live snapshot.");
            var result = new List<Placement>();
            var names = new HashSet<string>();
            foreach (var p in (JArray)snapshot["placements"])
            {
                var position = (JArray)p["position"];
                var orientation = (JArray)p["orientation"];
                var scale = (JArray)p["scale"];
                string name=(string)p["name"];
                if (string.IsNullOrEmpty(name) || !names.Add(name))
                    throw new InvalidOperationException("Missing or duplicate live entity name.");
                if (position.Count!=3 || orientation.Count!=4 || scale.Count!=3)
                    throw new InvalidOperationException("Invalid live transform dimensions.");
                string mesh = (string)p["mesh"];
                if (mesh != null && (!mesh.StartsWith("HeronBrushLive/preview/", StringComparison.Ordinal) || mesh.Contains("..") || mesh.Contains("\\")))
                    throw new InvalidOperationException("Mesh outside the live preview folder.");
                result.Add(new Placement(name, (string)p["classname"], mesh,
                    new Vector3(SnapshotNumber(position,0), SnapshotNumber(position,1), SnapshotNumber(position,2)),
                    new Quaternion(SnapshotNumber(orientation,0), SnapshotNumber(orientation,1), SnapshotNumber(orientation,2), SnapshotNumber(orientation,3)),
                    new Vector3(SnapshotNumber(scale,0), SnapshotNumber(scale,1), SnapshotNumber(scale,2)),
                    p["metadata"].ToString(Newtonsoft.Json.Formatting.None)));
            }
            Placements = result.ToArray();
            LiveEnvironment = snapshot["environment"] as JObject;
        }
