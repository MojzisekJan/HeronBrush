using System;
using System.Collections;
using System.Linq;
using Stride.Core.Mathematics;
using Stride.Engine;
using Stride.Physics;

namespace StrideMotion
{
    // Native check of Heron Brush Stride exports. HERON_LEVEL picks the level by name;
    // the level's generated classes are found by reflection so one scene checks them all.
    public class HeronLevelCheck : SyncScript
    {
        int frame;
        dynamic level;
        Entity ball, walker;
        Vector3 moverStart;
        Entity mover;
        string trigger = Nonempty("HERON_TRIGGER"), target = Nonempty("HERON_TARGET");
        static string Nonempty(string key) { var v = Environment.GetEnvironmentVariable(key); return string.IsNullOrEmpty(v) ? null : v; }
        public override void Start()
        {
            var scene = SceneSystem.SceneInstance.RootScene;
            foreach (var e in scene.Entities.Where(e => e.Name == "Ground" || e.Name == "Sphere").ToList()) scene.Entities.Remove(e);
            var name = Environment.GetEnvironmentVariable("HERON_LEVEL") ?? "motion_lab";
            var types = typeof(HeronLevelCheck).Assembly.GetTypes();
            var levelType = types.First(t => t.Name == "HeronBrushStrideLevel" && (string)t.GetField("Name").GetValue(null) == name);
            var physicsType = types.First(t => t.Name == "HeronBrushStridePhysics" && t.Namespace == levelType.Namespace);
            level = levelType.GetMethods().First(m => m.Name == "Build" && m.GetParameters()[0].ParameterType == typeof(Stride.Core.Serialization.Contents.ContentManager)).Invoke(null, new object[] { Content, null });
            var colliders = (IDictionary)physicsType.GetMethod("AddColliders").Invoke(null, new object[] { level });
            if (Nonempty("HERON_ENV") != null)
            {
                foreach (var e in scene.Entities.Where(e => e.Name == "Skybox" || e.Name == "Directional light").ToList()) scene.Entities.Remove(e);
                foreach (var e in (Entity[])levelType.GetMethod("CreateEnvironment").Invoke(null, new object[] { Content })) scene.Entities.Add(e);
            }
            Entity root = level.Root;
            scene.Entities.Add(root);
            var bounds = BoundingBox.Empty;
            foreach (var e in root.GetChildren())
            {
                var model = e.Get<ModelComponent>()?.Model;
                if (model == null) continue;
                e.Transform.UpdateWorldMatrix();
                foreach (var mesh in model.Meshes)
                {
                    var box = mesh.BoundingBox;
                    foreach (var corner in box.GetCorners()) bounds = BoundingBox.Merge(bounds, new BoundingBox(Vector3.TransformCoordinate(corner, e.Transform.WorldMatrix), Vector3.TransformCoordinate(corner, e.Transform.WorldMatrix)));
                }
            }
            var centre = bounds.Center; var radius = bounds.Extent.Length();
            var colliderList = colliders.Values.Cast<PhysicsComponent>().ToList();
            Console.WriteLine($"HERON level={name} entities={((IDictionary)level.Entities).Count} models={root.GetChildren().Count(e => e.Get<ModelComponent>() != null)} colliders={colliderList.Count} " +
                $"static={colliderList.Count(c => c is StaticColliderComponent s && !s.IsTrigger)} triggers={colliderList.Count(c => c is StaticColliderComponent s && s.IsTrigger)} kinematic={colliderList.Count(c => c is RigidbodyComponent)} " +
                $"animations={((IDictionary)level.Animations).Count} bounds={bounds.Minimum}..{bounds.Maximum}");
            ball = new Entity("ball") { Transform = { Position = new Vector3(centre.X, bounds.Maximum.Y + 2, centre.Z) } };
            ball.Add(new RigidbodyComponent { ColliderShape = new SphereColliderShape(false, 0.25f), Mass = 1 });
            scene.Entities.Add(ball);
            walker = new Entity("walker") { Transform = { Position = new Vector3(0, -1000, 0) } };
            scene.Entities.Add(walker);
            level.Activators.Add(walker);
            if (target != null) { mover = ((System.Collections.Generic.IReadOnlyDictionary<string, Entity>)level.Entities)[target]; moverStart = mover.Transform.Position; }
            var camera = scene.Entities.First(e => e.Name == "Camera");
            var eye = Nonempty("HERON_LOW") != null ? centre + new Vector3(radius * 0.1f, radius * 0.12f, radius * 0.6f) : centre + new Vector3(radius * 0.15f, radius * 0.55f, radius * 0.75f);
            camera.Transform.Position = eye;
            var dir = Vector3.Normalize(centre - eye);
            if (Nonempty("HERON_SUN") != null) { var toSun = (Vector3)levelType.GetField("ToSun").GetValue(null); dir = Vector3.Normalize(new Vector3(toSun.X, toSun.Y * 0.6f, toSun.Z)); eye = centre + new Vector3(0, radius * 0.4f, 0); camera.Transform.Position = eye; }
            camera.Transform.Rotation = Quaternion.RotationYawPitchRoll((float)Math.Atan2(-dir.X, -dir.Z), (float)Math.Asin(dir.Y), 0);
            var cam = camera.Get<CameraComponent>(); cam.FarClipPlane = radius * 10; cam.NearClipPlane = 0.05f;
            foreach (var c in camera.Components.OfType<BasicCameraController>().ToList()) camera.Components.Remove(c);
        }
        public override void Update()
        {
            frame++;
            if (frame == 20 && trigger != null)
            {
                var t = ((System.Collections.Generic.IReadOnlyDictionary<string, Entity>)level.Entities)[trigger];
                walker.Transform.Position = t.Transform.Position + new Vector3(0, 0.05f, 0);
            }
            if (frame == 240)
            {
                if (mover != null) Console.WriteLine($"HERON {target} moved={(mover.Transform.Position - moverStart).Length():F3} pos={mover.Transform.Position} rot={mover.Transform.Rotation}");
                var hits = new System.Collections.Generic.List<HitResult>();
                this.GetSimulation().RaycastPenetrating(ball.Transform.Position + new Vector3(0, 3, 0), ball.Transform.Position - new Vector3(0, 30, 0), hits);
                Console.WriteLine($"HERON ball y={ball.Transform.Position.Y:F3} below={string.Join(",", hits.Select(h => $"{h.Collider.Entity.Name}@{h.Point.Y:F3}"))}");
                Console.WriteLine("HERON done");
            }
        }
    }
}
