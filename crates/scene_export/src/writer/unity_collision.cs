        static void AddCollision(GameObject node, Shape shape, bool sensor, string folder) {
            if (shape.points == null || shape.points.Length % 3 != 0) throw new InvalidDataException("Invalid collision points.");
            var points = new Vector3[shape.points.Length / 3];
            for(int i=0;i<points.Length;i++) points[i]=new Vector3(-shape.points[i*3],shape.points[i*3+1],shape.points[i*3+2]);
            if(points.Length<3) return;
            var indices=new List<int>();
            if(shape.convex) { for(int i=1;i+1<points.Length;i++){indices.Add(0);indices.Add(i+1);indices.Add(i);} }
            // Collision triangles arrive clockwise in the Godot frame. Reflecting
            // X already gives PhysX outward faces; reversing again points inward.
            else { for(int i=0;i+2<points.Length;i+=3){indices.Add(i);indices.Add(i+1);indices.Add(i+2);} }
            var mesh = new Mesh { name="HeronBrushCollision", indexFormat=UnityEngine.Rendering.IndexFormat.UInt32 };
            mesh.vertices=points; mesh.SetTriangles(indices,0); mesh.RecalculateBounds();
            // Persist collider meshes so saving/reopening the Unity scene keeps collision.
            string hash;
            using(var sha = System.Security.Cryptography.SHA256.Create())
                hash = System.BitConverter.ToString(sha.ComputeHash(System.Text.Encoding.UTF8.GetBytes("unity-collision-v2\n"+JsonUtility.ToJson(shape)))).Replace("-", "");
            if (!AssetDatabase.IsValidFolder(folder)) {
                Directory.CreateDirectory(folder);
                AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
            }
            var path=$"{folder}/collision_{hash}.asset";
            var saved=AssetDatabase.LoadAssetAtPath<Mesh>(path);
            if(saved) { Object.DestroyImmediate(mesh); }
            else { AssetDatabase.CreateAsset(mesh,path); saved=mesh; }
            var collider=node.AddComponent<MeshCollider>();
            collider.convex=shape.convex || sensor; collider.sharedMesh=saved; collider.isTrigger=sensor;
        }
